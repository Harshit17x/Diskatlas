use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use rusqlite::{Connection, OpenFlags};
use tracing::{info, warn};
use crate::error::{AppError, Result};
use crate::models::database::{DatabaseIntegrityReport, DatabaseMaintenanceReport};

#[derive(Clone)]
pub struct DbPool {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
    is_memory: bool,
}

impl DbPool {
    /// Initializes a SQLite connection at the given path.
    /// Enforces WAL mode, foreign keys, autocheckpoint, and busy timeout for safe concurrent operations.
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| AppError::Io {
                    path: parent.to_string_lossy().to_string(),
                    source: e,
                })?;
            }
        }

        let conn = Connection::open(path)?;

        // Enable incremental auto-vacuum on newly initialized databases so pruned scan
        // sessions return freed pages to the OS without requiring full-file VACUUM copies.
        let initial_pages: i64 = conn
            .query_row("PRAGMA page_count;", [], |r| r.get(0))
            .unwrap_or(1);
        if initial_pages == 0 {
            let _ = conn.pragma_update(None, "auto_vacuum", "INCREMENTAL");
        }

        // Configure SQLite performance, memory caching, and concurrency pragmas
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "wal_autocheckpoint", 2000)?;
        conn.pragma_update(None, "cache_size", -64000)?; // 64 MB page cache in memory
        conn.pragma_update(None, "temp_store", "MEMORY")?; // Store temp tables & indexes in RAM
        conn.pragma_update(None, "mmap_size", 268435456)?; // 256 MB memory-mapped I/O

        info!("SQLite connected at '{}' with WAL mode enabled", path.display());

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path.to_path_buf(),
            is_memory: false,
        })
    }

    /// Creates an in-memory SQLite database instance (primarily for automated testing).
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
            is_memory: true,
        })
    }

    /// Access the mutex-guarded primary/write SQLite connection
    pub fn conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Poison-tolerant lock helper for the write/primary connection.
    pub fn lock_conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| {
            warn!("Recovering from poisoned SQLite mutex");
            e.into_inner()
        })
    }

    /// Non-blocking lock helper for callers that can defer DB registration until the first write.
    pub fn try_lock_conn(&self) -> Option<MutexGuard<'_, Connection>> {
        match self.conn.try_lock() {
            Ok(guard) => Some(guard),
            Err(std::sync::TryLockError::Poisoned(e)) => {
                warn!("Recovering from poisoned SQLite mutex");
                Some(e.into_inner())
            }
            Err(std::sync::TryLockError::WouldBlock) => None,
        }
    }

    /// Executes a read-only closure using a concurrent WAL reader connection for file-backed DBs,
    /// avoiding contention with long-running write transactions. Falls back to the shared
    /// connection for `:memory:` test databases.
    pub fn with_read_conn<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        if self.is_memory || !self.db_path.exists() {
            let conn = self.lock_conn();
            return f(&conn);
        }

        match Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(read_conn) => {
                let _ = read_conn.pragma_update(None, "busy_timeout", 5000);
                let _ = read_conn.pragma_update(None, "query_only", "ON");
                let _ = read_conn.pragma_update(None, "cache_size", -16000);
                let _ = read_conn.pragma_update(None, "temp_store", "MEMORY");
                let _ = read_conn.pragma_update(None, "mmap_size", 268435456);
                f(&read_conn)
            }
            Err(_) => {
                let conn = self.lock_conn();
                f(&conn)
            }
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn file_sizes(&self) -> (u64, u64) {
        if self.is_memory {
            return (0, 0);
        }
        let db_size = std::fs::metadata(&self.db_path).map(|m| m.len()).unwrap_or(0);
        let wal_path = PathBuf::from(format!("{}-wal", self.db_path.display()));
        let wal_size = std::fs::metadata(&wal_path).map(|m| m.len()).unwrap_or(0);
        (db_size, wal_size)
    }

    /// Truncates the SQLite Write-Ahead Log back into the main database file.
    pub fn checkpoint_wal(&self) -> Result<()> {
        if self.is_memory {
            return Ok(());
        }
        let conn = self.lock_conn();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }

    /// Runs `PRAGMA integrity_check` and returns size/status diagnostics.
    pub fn verify_integrity(&self) -> Result<DatabaseIntegrityReport> {
        let details: Vec<String> = self.with_read_conn(|conn| {
            let mut stmt = conn.prepare("PRAGMA integrity_check(10);")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })?;

        let ok = details.len() == 1 && details[0].eq_ignore_ascii_case("ok");
        let (db_size_bytes, wal_size_bytes) = self.file_sizes();

        Ok(DatabaseIntegrityReport {
            ok,
            details,
            db_size_bytes,
            wal_size_bytes,
        })
    }

    /// Checkpoints WAL and compacts the SQLite database with `VACUUM`.
    pub fn vacuum_database(&self) -> Result<DatabaseMaintenanceReport> {
        let (prev_db, prev_wal) = self.file_sizes();
        let previous_bytes = prev_db + prev_wal;

        if !self.is_memory {
            let conn = self.lock_conn();
            conn.execute_batch(
                "PRAGMA wal_checkpoint(TRUNCATE);
                 PRAGMA auto_vacuum = INCREMENTAL;
                 VACUUM;
                 PRAGMA wal_checkpoint(TRUNCATE);",
            )?;
        }

        let (curr_db, curr_wal) = self.file_sizes();
        let current_bytes = curr_db + curr_wal;
        let reclaimed_bytes = previous_bytes.saturating_sub(current_bytes);

        Ok(DatabaseMaintenanceReport {
            ok: true,
            previous_bytes,
            current_bytes,
            reclaimed_bytes,
            message: format!(
                "Database compacted via WAL checkpoint and VACUUM (reclaimed {} bytes)",
                reclaimed_bytes
            ),
        })
    }

    /// Count all user tables present in SQLite master
    pub fn count_tables(&self) -> Result<usize> {
        self.with_read_conn(Self::count_tables_with_conn)
    }

    /// Count tables using an existing connection reference without re-acquiring mutex
    pub fn count_tables_with_conn(conn: &Connection) -> Result<usize> {
        let mut stmt = conn.prepare(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        )?;
        let count: usize = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    /// Ensures `idx_dirs_session_path_unique` exists before directory upserts (`ON CONFLICT`),
    /// deduplicating duplicate `(session_id, path)` pairs first if needed.
    pub fn ensure_directories_unique_index(conn: &Connection) -> Result<()> {
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_dirs_session_path_unique')",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if exists {
            return Ok(());
        }

        info!("Creating unique directory index idx_dirs_session_path_unique...");
        conn.execute_batch(
            "DELETE FROM directories
             WHERE id IN (
                 SELECT d.id
                 FROM directories d
                 JOIN (
                     SELECT session_id, path, MAX(id) AS keep_id
                     FROM directories
                     GROUP BY session_id, path
                     HAVING COUNT(*) > 1
                 ) dup ON d.session_id = dup.session_id AND d.path = dup.path AND d.id != dup.keep_id
             );
             CREATE UNIQUE INDEX IF NOT EXISTS idx_dirs_session_path_unique
                 ON directories(session_id, path);",
        )?;
        Ok(())
    }

    /// Executes deferred large-DB integrity & summary repair tasks asynchronously after startup
    /// so the UI window opens immediately even on multi-gigabyte databases.
    pub fn run_startup_maintenance(&self) -> Result<()> {
        if self.is_memory {
            return Ok(());
        }

        let (needs_dirs_unique, needs_orphans, needs_summaries, needs_compact) =
            self.with_read_conn(|conn| {
                let flag = |key: &str| -> bool {
                    conn.query_row(
                        "SELECT value FROM app_settings WHERE key = ?1",
                        [key],
                        |r| r.get::<_, String>(0),
                    )
                    .map(|v| v == "1")
                    .unwrap_or(false)
                };
                Ok((
                    flag("maintenance:dirs_unique_pending"),
                    flag("maintenance:orphans_pending"),
                    flag("maintenance:summaries_pending"),
                    flag("maintenance:compact_pending"),
                ))
            })?;

        if needs_dirs_unique {
            // Inspect index and duplicate directory IDs on the read connection first so long-running
            // scans or drive syncs are not blocked on `lock_conn()` during multi-million-row checks.
            let (idx_exists, dup_ids): (bool, Vec<i64>) = self.with_read_conn(|conn| {
                let exists: bool = conn
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_dirs_session_path_unique')",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap_or(false);
                if exists {
                    return Ok((true, Vec::new()));
                }
                let mut stmt = conn.prepare(
                    "SELECT d.id
                     FROM directories d
                     JOIN (
                         SELECT session_id, path, MAX(id) AS keep_id
                         FROM directories
                         GROUP BY session_id, path
                         HAVING COUNT(*) > 1
                     ) dup ON d.session_id = dup.session_id AND d.path = dup.path AND d.id != dup.keep_id",
                )?;
                let ids = stmt.query_map([], |r| r.get::<_, i64>(0))?;
                Ok((false, ids.filter_map(|r| r.ok()).collect()))
            })?;

            let conn = self.lock_conn();
            if !idx_exists {
                info!(
                    "Creating unique directory index idx_dirs_session_path_unique ({} duplicate rows to prune)...",
                    dup_ids.len()
                );
                if !dup_ids.is_empty() {
                    let tx = conn.unchecked_transaction()?;
                    {
                        let mut del = tx.prepare_cached("DELETE FROM directories WHERE id = ?1")?;
                        for id in dup_ids {
                            del.execute([id])?;
                        }
                    }
                    tx.commit()?;
                }
                conn.execute_batch(
                    "CREATE UNIQUE INDEX IF NOT EXISTS idx_dirs_session_path_unique
                     ON directories(session_id, path);",
                )?;
            }
            let _ = conn.execute(
                "DELETE FROM app_settings WHERE key = 'maintenance:dirs_unique_pending'",
                [],
            );
        }

        if needs_orphans {
            let orphan_file_sids = self.find_orphan_sessions_via_skip_scan("files")?;
            let orphan_dir_sids = self.find_orphan_sessions_via_skip_scan("directories")?;

            let conn = self.lock_conn();
            for sid in orphan_file_sids {
                let _ = conn.execute("DELETE FROM files WHERE session_id = ?1", [&sid]);
            }
            for sid in orphan_dir_sids {
                let _ = conn.execute("DELETE FROM directories WHERE session_id = ?1", [&sid]);
            }
            let _ = conn.execute_batch(
                "DELETE FROM scan_errors WHERE session_id IS NOT NULL AND session_id NOT IN (SELECT id FROM scan_sessions);
                 DELETE FROM classification_summaries WHERE session_id IS NOT NULL AND session_id NOT IN (SELECT id FROM scan_sessions);
                 DELETE FROM category_summaries WHERE session_id IS NOT NULL AND session_id NOT IN (SELECT id FROM scan_sessions);
                 DELETE FROM developer_projects WHERE session_id IS NOT NULL AND session_id NOT IN (SELECT id FROM scan_sessions);
                 DELETE FROM developer_ecosystem_summaries WHERE session_id IS NOT NULL AND session_id NOT IN (SELECT id FROM scan_sessions);
                 DELETE FROM storage_recommendations WHERE session_id IS NOT NULL AND session_id NOT IN (SELECT id FROM scan_sessions);
                 DELETE FROM app_settings WHERE key = 'maintenance:orphans_pending';",
            );
        }

        if needs_summaries {
            // Find completed sessions needing canonical category_summaries without holding write lock.
            let target_sessions: Vec<String> = self.with_read_conn(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT s.id
                     FROM scan_sessions s
                     WHERE s.status IN ('completed', 'completed_with_errors', 'completedWithErrors')
                       AND COALESCE(s.total_files, 0) > 0
                       AND (
                           EXISTS (
                               SELECT 1 FROM category_summaries cs
                               WHERE cs.session_id = s.id
                                 AND cs.category IN ('code', 'executables', 'other', 'system')
                           )
                           OR NOT EXISTS (
                               SELECT 1 FROM category_summaries cs
                               WHERE cs.session_id = s.id
                           )
                       )",
                )?;
                let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
                Ok(rows.filter_map(|r| r.ok()).collect())
            })?;

            for sid in target_sessions {
                // Aggregate via concurrent WAL reader so the main write connection stays unblocked.
                let rows: Vec<(String, i64, i64)> = self.with_read_conn(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT COALESCE(NULLIF(category, ''), 'unknown'), COUNT(*), COALESCE(SUM(size_bytes), 0)
                         FROM files
                         WHERE session_id = ?1
                         GROUP BY COALESCE(NULLIF(category, ''), 'unknown')",
                    )?;
                    let mapped = stmt.query_map([&sid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
                    Ok(mapped.filter_map(|r| r.ok()).collect())
                })?;

                let conn = self.lock_conn();
                let tx = conn.unchecked_transaction()?;
                tx.execute("DELETE FROM category_summaries WHERE session_id = ?1", [&sid])?;
                {
                    let mut ins = tx.prepare_cached(
                        "INSERT OR REPLACE INTO category_summaries (session_id, category, file_count, total_bytes)
                         VALUES (?1, ?2, ?3, ?4)",
                    )?;
                    for (cat, cnt, bytes) in rows {
                        ins.execute(rusqlite::params![&sid, cat, cnt, bytes])?;
                    }
                }
                tx.commit()?;
            }

            let conn = self.lock_conn();
            let _ = conn.execute(
                "DELETE FROM app_settings WHERE key = 'maintenance:summaries_pending'",
                [],
            );
        }

        if needs_compact {
            let conn = self.lock_conn();
            info!("Dropping redundant/unused SQLite indexes to shrink per-scan database footprint...");
            conn.execute_batch(
                "DROP INDEX IF EXISTS idx_dirs_session_path;
                 DROP INDEX IF EXISTS idx_files_session_hidden;
                 DROP INDEX IF EXISTS idx_files_session_ext;
                 DROP INDEX IF EXISTS idx_files_session_subcat;
                 DROP INDEX IF EXISTS idx_files_session_app;
                 DELETE FROM app_settings WHERE key = 'maintenance:compact_pending';",
            )?;
        }

        // Automatically prune superseded raw file/directory trees from older scans on startup
        // (preserving lightweight scan_sessions, category_summaries, and storage_snapshots)
        // and reclaim freed pages on disk when significant bloat is present.
        let (pruned, reclaimed) =
            self.prune_superseded_scans_and_reclaim(None, true, true)?;
        if pruned > 0 || reclaimed > 0 {
            info!(
                "Startup retention pruned {} superseded session tree(s) and reclaimed {} bytes",
                pruned, reclaimed
            );
        }

        if needs_dirs_unique || needs_orphans || needs_summaries || needs_compact {
            info!("Deferred database startup maintenance complete");
        }
        Ok(())
    }

    /// Identifies superseded scan sessions that still store bulky raw rows in `files`,
    /// `directories`, `scan_errors`, `classification_summaries`, `developer_projects`,
    /// `developer_ecosystem_summaries`, or `storage_recommendations`.
    ///
    /// Retention rule per `drive_id`:
    /// - Never prunes currently `running` or `paused` sessions.
    /// - Retains full row-level details ONLY for the newest completed session (preferring
    ///   sessions with `total_files > 0`) on that drive, or—if no completed session exists—
    ///   the newest non-empty `interrupted`/`cancelled` fallback session when requested.
    /// - Lightweight history records (`scan_sessions`, `category_summaries`, `storage_snapshots`)
    ///   are ALWAYS preserved.
    pub fn find_superseded_detail_session_ids(
        &self,
        drive_filter: Option<&str>,
        preserve_latest_fallback_if_no_completed: bool,
    ) -> Result<Vec<String>> {
        if let Some(id) = drive_filter {
            if id.trim().is_empty() {
                return Ok(Vec::new());
            }
        }
        self.with_read_conn(|conn| {
            use std::collections::HashMap;

            #[derive(Clone)]
            struct SessRow {
                id: String,
                drive_id: String,
                status: String,
                total_files: i64,
            }

            let rows_vec: Vec<SessRow> = if let Some(did) = drive_filter {
                let mut stmt = conn.prepare(
                    "SELECT id, drive_id, status, COALESCE(total_files, 0)
                     FROM scan_sessions
                     WHERE drive_id = ?1
                     ORDER BY (CASE WHEN COALESCE(total_files, 0) > 0 THEN 0 ELSE 1 END) ASC,
                              started_at DESC,
                              rowid DESC",
                )?;
                let mapped = stmt.query_map([did], |r| {
                    Ok(SessRow {
                        id: r.get(0)?,
                        drive_id: r.get(1)?,
                        status: r.get(2)?,
                        total_files: r.get(3)?,
                    })
                })?;
                mapped.filter_map(|r| r.ok()).collect()
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, drive_id, status, COALESCE(total_files, 0)
                     FROM scan_sessions
                     ORDER BY (CASE WHEN COALESCE(total_files, 0) > 0 THEN 0 ELSE 1 END) ASC,
                              started_at DESC,
                              rowid DESC",
                )?;
                let mapped = stmt.query_map([], |r| {
                    Ok(SessRow {
                        id: r.get(0)?,
                        drive_id: r.get(1)?,
                        status: r.get(2)?,
                        total_files: r.get(3)?,
                    })
                })?;
                mapped.filter_map(|r| r.ok()).collect()
            };

            let mut by_drive: HashMap<String, Vec<SessRow>> = HashMap::new();
            for row in rows_vec {
                by_drive.entry(row.drive_id.clone()).or_default().push(row);
            }

            let is_completed = |st: &str| {
                matches!(
                    st,
                    "completed" | "completed_with_errors" | "completedWithErrors"
                )
            };
            let is_active = |st: &str| matches!(st, "running" | "paused");
            let is_fallback = |st: &str, tf: i64| {
                tf > 0 && matches!(st, "interrupted" | "cancelled")
            };

            let mut candidates: Vec<String> = Vec::new();

            for (_drive, list) in by_drive {
                let keep_id: Option<String> = list
                    .iter()
                    .find(|s| is_completed(&s.status))
                    .map(|s| s.id.clone())
                    .or_else(|| {
                        if preserve_latest_fallback_if_no_completed {
                            list.iter()
                                .find(|s| is_fallback(&s.status, s.total_files))
                                .map(|s| s.id.clone())
                        } else {
                            None
                        }
                    });

                for s in list {
                    if is_active(&s.status) {
                        continue;
                    }
                    if keep_id.as_deref() == Some(s.id.as_str()) {
                        continue;
                    }

                    // Fast O(log N) existence check via session_id prefix indexes
                    let has_heavy_rows: bool = conn
                        .query_row(
                            "SELECT
                                EXISTS(SELECT 1 FROM files WHERE session_id = ?1 LIMIT 1)
                                OR EXISTS(SELECT 1 FROM directories WHERE session_id = ?1 LIMIT 1)
                                OR EXISTS(SELECT 1 FROM scan_errors WHERE session_id = ?1 LIMIT 1)
                                OR EXISTS(SELECT 1 FROM classification_summaries WHERE session_id = ?1 LIMIT 1)
                                OR EXISTS(SELECT 1 FROM developer_projects WHERE session_id = ?1 LIMIT 1)
                                OR EXISTS(SELECT 1 FROM developer_ecosystem_summaries WHERE session_id = ?1 LIMIT 1)
                                OR EXISTS(SELECT 1 FROM storage_recommendations WHERE session_id = ?1 LIMIT 1)",
                            [&s.id],
                            |r| r.get(0),
                        )
                        .unwrap_or(false);

                    if has_heavy_rows {
                        candidates.push(s.id);
                    }
                }
            }

            Ok(candidates)
        })
    }

    /// Removes heavy row-level scan data for a single session while preserving its
    /// `scan_sessions`, `category_summaries`, and `storage_snapshots` history records.
    pub fn prune_session_heavy_data(&self, session_id: &str) -> Result<()> {
        let mut conn = self.lock_conn();
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM files WHERE session_id = ?1", [session_id])?;
        tx.execute("DELETE FROM directories WHERE session_id = ?1", [session_id])?;
        tx.execute("DELETE FROM scan_errors WHERE session_id = ?1", [session_id])?;
        tx.execute("DELETE FROM classification_summaries WHERE session_id = ?1", [session_id])?;
        tx.execute("DELETE FROM developer_projects WHERE session_id = ?1", [session_id])?;
        tx.execute(
            "DELETE FROM developer_ecosystem_summaries WHERE session_id = ?1",
            [session_id],
        )?;
        tx.execute("DELETE FROM storage_recommendations WHERE session_id = ?1", [session_id])?;
        tx.execute(
            "DELETE FROM app_settings WHERE key IN (?1, ?2)",
            rusqlite::params![
                format!("dev_analyzed:{}", session_id),
                format!("intel_generated:{}", session_id)
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Reclaims unused SQLite freelist pages and truncates the WAL journal back to 0 bytes.
    ///
    /// - On `auto_vacuum = INCREMENTAL` (`2`) databases, runs fast `PRAGMA incremental_vacuum`
    ///   without copying the file or needing 2x free disk space.
    /// - On legacy `auto_vacuum = NONE` (`0`) databases, upgrades once to `INCREMENTAL` via
    ///   `VACUUM` only when `allow_one_time_full_vacuum` is enabled, significant wasted space
    ///   exists (`>= 32 MB` or `>= 20%` freelist), and the host volume has safe free headroom.
    pub fn reclaim_free_pages(&self, allow_one_time_full_vacuum: bool) -> Result<u64> {
        if self.is_memory {
            return Ok(0);
        }

        let (prev_db, prev_wal) = self.file_sizes();
        let before_total = prev_db + prev_wal;

        {
            let conn = self.lock_conn();
            let auto_vac: i64 = conn.query_row("PRAGMA auto_vacuum;", [], |r| r.get(0)).unwrap_or(0);
            let freelist: i64 = conn.query_row("PRAGMA freelist_count;", [], |r| r.get(0)).unwrap_or(0);
            let page_count: i64 = conn.query_row("PRAGMA page_count;", [], |r| r.get(0)).unwrap_or(0);
            let page_size: i64 = conn.query_row("PRAGMA page_size;", [], |r| r.get(0)).unwrap_or(4096);

            let freelist_bytes = (freelist.max(0) as u64).saturating_mul(page_size.max(0) as u64);
            let live_bytes = (page_count.saturating_sub(freelist).max(0) as u64)
                .saturating_mul(page_size.max(0) as u64);

            if auto_vac == 2 {
                if freelist > 0 {
                    conn.execute_batch("PRAGMA incremental_vacuum; PRAGMA wal_checkpoint(TRUNCATE);")?;
                } else {
                    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
                }
            } else if allow_one_time_full_vacuum
                && freelist > 0
                && (freelist_bytes >= 32 * 1024 * 1024
                    || freelist.saturating_mul(5) >= page_count.max(1))
                && Self::has_vacuum_disk_headroom(&self.db_path, live_bytes)
            {
                info!(
                    "Converting legacy SQLite DB to incremental auto_vacuum and reclaiming {} bytes of free pages...",
                    freelist_bytes
                );
                conn.execute_batch(
                    "PRAGMA wal_checkpoint(TRUNCATE);
                     PRAGMA auto_vacuum = INCREMENTAL;
                     VACUUM;
                     PRAGMA wal_checkpoint(TRUNCATE);",
                )?;
            } else {
                conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
            }
        }

        let (curr_db, curr_wal) = self.file_sizes();
        let after_total = curr_db + curr_wal;
        Ok(before_total.saturating_sub(after_total))
    }

    fn has_vacuum_disk_headroom(db_path: &Path, required_live_bytes: u64) -> bool {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let canonical = std::fs::canonicalize(db_path).unwrap_or_else(|_| db_path.to_path_buf());
        let mut best_match: Option<(usize, u64)> = None;
        for d in disks.list() {
            let mount = d.mount_point();
            if canonical.starts_with(mount) {
                let len = mount.as_os_str().len();
                if best_match.map_or(true, |(best_len, _)| len > best_len) {
                    best_match = Some((len, d.available_space()));
                }
            }
        }
        match best_match {
            // Require compacted DB size + 512 MB safety margin before running full VACUUM
            Some((_, avail)) => avail >= required_live_bytes.saturating_add(512 * 1024 * 1024),
            None => true,
        }
    }

    /// Prunes superseded raw scan detail trees and reclaims freed pages on disk.
    pub fn prune_superseded_scans_and_reclaim(
        &self,
        drive_filter: Option<&str>,
        preserve_latest_fallback_if_no_completed: bool,
        allow_one_time_full_vacuum: bool,
    ) -> Result<(usize, u64)> {
        let sids = self.find_superseded_detail_session_ids(
            drive_filter,
            preserve_latest_fallback_if_no_completed,
        )?;
        let mut pruned_count = 0usize;
        for sid in &sids {
            self.prune_session_heavy_data(sid)?;
            pruned_count += 1;
        }
        let reclaimed = self.reclaim_free_pages(allow_one_time_full_vacuum)?;
        Ok((pruned_count, reclaimed))
    }

    fn find_orphan_sessions_via_skip_scan(&self, table: &str) -> Result<Vec<String>> {
        self.with_read_conn(|conn| {
            let sql = format!(
                "WITH RECURSIVE sids(sid) AS (
                     SELECT MIN(session_id) FROM {t}
                     UNION ALL
                     SELECT (SELECT MIN(session_id) FROM {t} WHERE session_id > sids.sid)
                     FROM sids WHERE sids.sid IS NOT NULL
                 )
                 SELECT sid FROM sids
                 WHERE sid IS NOT NULL
                   AND NOT EXISTS (SELECT 1 FROM scan_sessions WHERE id = sid)",
                t = table
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrator::{Migrator, MIGRATIONS};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(1);

    struct TempTestDir(std::path::PathBuf);

    impl TempTestDir {
        fn new(prefix: &str) -> Self {
            let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!(
                "{}_{}_{}",
                prefix,
                std::process::id(),
                seq
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn seed_v8_large_db(db_path: &Path) {
        let conn = Connection::open(db_path).unwrap();
        for migration in &MIGRATIONS[..8] {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
                (migration.version, migration.name),
            )
            .unwrap();
        }
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
             VALUES ('drv-1', 'Main', '/tmp', 'APFS', 1000, 500)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files)
             VALUES ('sess-1', 'drv-1', '/tmp', 'completed', 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO files (id, session_id, path, name, parent_path, size_bytes, category)
             VALUES
                (50000, 'sess-1', '/tmp/main.rs', 'main.rs', '/tmp', 4096, 'developer'),
                (50001, 'ghost-session', '/tmp/orphan.bin', 'orphan.bin', '/tmp', 999, 'other')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO directories (id, session_id, path, name, parent_path, file_count, dir_count, total_size_bytes)
             VALUES
                (1, 'sess-1', '/tmp/dup', 'dup', '/tmp', 1, 0, 100),
                (2, 'sess-1', '/tmp/dup', 'dup', '/tmp', 1, 0, 200),
                (3, 'ghost-session', '/tmp/orphan_dir', 'orphan_dir', '/tmp', 0, 0, 50)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
             VALUES ('sess-1', 'code', 1, 4096)",
            [],
        )
        .unwrap();
    }

    #[test]
    fn test_startup_maintenance_repairs_large_deferred_db() {
        let dir = TempTestDir::new("diskatlas_maint_test");
        let db_path = dir.0.join("diskatlas.db");
        seed_v8_large_db(&db_path);

        let pool = DbPool::new(&db_path).unwrap();
        {
            let mut conn = pool.lock_conn();
            let version = Migrator::run(&mut conn).unwrap();
            assert_eq!(version, 12);
        }

        pool.run_startup_maintenance().unwrap();

        let conn = pool.lock_conn();
        let idx_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_dirs_session_path_unique')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(idx_exists);

        let dropped_dup_idx: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_dirs_session_path')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!dropped_dup_idx, "duplicate idx_dirs_session_path should be dropped");

        let dropped_hidden_idx: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'index' AND name = 'idx_files_session_hidden')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!dropped_hidden_idx, "unused idx_files_session_hidden should be dropped");

        let dir_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM directories", [], |r| r.get(0))
            .unwrap();
        assert_eq!(dir_count, 1);

        let kept_size: i64 = conn
            .query_row("SELECT total_size_bytes FROM directories WHERE session_id = 'sess-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(kept_size, 200);

        let file_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(file_count, 1);

        let (cat, cnt, bytes): (String, i64, i64) = conn
            .query_row(
                "SELECT category, file_count, total_bytes FROM category_summaries WHERE session_id = 'sess-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(cat, "developer");
        assert_eq!(cnt, 1);
        assert_eq!(bytes, 4096);

        let pending_flags: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM app_settings
                 WHERE key IN ('maintenance:dirs_unique_pending', 'maintenance:orphans_pending', 'maintenance:summaries_pending', 'maintenance:compact_pending')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending_flags, 0);
    }

    #[test]
    fn test_ensure_directories_unique_index_dedups_and_allows_upsert() {
        let dir = TempTestDir::new("diskatlas_idx_guard_test");
        let db_path = dir.0.join("diskatlas.db");
        seed_v8_large_db(&db_path);

        let pool = DbPool::new(&db_path).unwrap();
        {
            let mut conn = pool.lock_conn();
            assert_eq!(Migrator::run(&mut conn).unwrap(), 12);
            DbPool::ensure_directories_unique_index(&conn).unwrap();

            conn.execute(
                "INSERT INTO directories (
                    session_id, path, name, parent_path, file_count, dir_count, total_size_bytes
                 ) VALUES ('sess-1', '/tmp/dup', 'dup', '/tmp', 2, 0, 4096)
                 ON CONFLICT(session_id, path) DO UPDATE SET
                    file_count = excluded.file_count,
                    total_size_bytes = excluded.total_size_bytes",
                [],
            )
            .unwrap();

            let updated_bytes: i64 = conn
                .query_row(
                    "SELECT total_size_bytes FROM directories WHERE session_id = 'sess-1' AND path = '/tmp/dup'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(updated_bytes, 4096);
        }

        // Subsequent background maintenance remains idempotent and clears pending flags.
        pool.run_startup_maintenance().unwrap();
    }

    #[test]
    fn test_incremental_auto_vacuum_and_superseded_session_pruning_reclaims_space() {
        let dir = TempTestDir::new("diskatlas_retention_reclaim");
        let db_path = dir.0.join("diskatlas.db");
        let pool = DbPool::new(&db_path).unwrap();

        {
            let mut conn = pool.lock_conn();
            let auto_vac: i64 = conn.query_row("PRAGMA auto_vacuum;", [], |r| r.get(0)).unwrap();
            assert_eq!(auto_vac, 2, "New DBs should default to INCREMENTAL auto_vacuum");
            assert_eq!(Migrator::run(&mut conn).unwrap(), 12);

            conn.execute(
                "INSERT INTO drives (id, label, mount_point, filesystem, total_bytes, available_bytes)
                 VALUES ('MAC', 'Macintosh HD', '/', 'apfs', 1000000, 500000)",
                [],
            )
            .unwrap();

            conn.execute(
                "INSERT INTO scan_sessions (id, drive_id, root_path, status, total_files, total_directories, total_bytes, started_at)
                 VALUES
                    ('sess-old', 'MAC', '/', 'completed', 500, 1, 500000, '2026-11-01 10:00:00'),
                    ('sess-new', 'MAC', '/', 'completed', 10, 1, 10000, '2026-11-02 10:00:00'),
                    ('sess-cancel', 'MAC', '/', 'cancelled', 200, 1, 20000, '2026-11-03 10:00:00')",
                [],
            )
            .unwrap();

            conn.execute(
                "INSERT INTO category_summaries (session_id, category, file_count, total_bytes)
                 VALUES ('sess-old', 'documents', 500, 500000), ('sess-new', 'documents', 10, 10000)",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO storage_snapshots (id, session_id, drive_id, label, root_path, total_files, total_directories, total_bytes)
                 VALUES ('snap-old', 'sess-old', 'MAC', 'Scan Completed', '/', 500, 1, 500000)",
                [],
            )
            .unwrap();

            let tx = conn.unchecked_transaction().unwrap();
            {
                let mut ins = tx
                    .prepare_cached(
                        "INSERT INTO files (session_id, path, name, parent_path, size_bytes, category)
                         VALUES (?1, ?2, 'f.bin', '/tmp', 1024, 'documents')",
                    )
                    .unwrap();
                for i in 0..3000 {
                    ins.execute(rusqlite::params!["sess-old", format!("/tmp/old_{}_{}.bin", i, "x".repeat(48))])
                        .unwrap();
                }
                for i in 0..200 {
                    ins.execute(rusqlite::params!["sess-cancel", format!("/tmp/cancel_{}.bin", i)])
                        .unwrap();
                }
                for i in 0..10 {
                    ins.execute(rusqlite::params!["sess-new", format!("/tmp/new_{}.bin", i)])
                        .unwrap();
                }
            }
            tx.commit().unwrap();
        }

        pool.checkpoint_wal().unwrap();
        let (before_db, before_wal) = pool.file_sizes();
        let before_total = before_db + before_wal;

        let (pruned, reclaimed) = pool
            .prune_superseded_scans_and_reclaim(Some("MAC"), false, false)
            .unwrap();
        assert_eq!(pruned, 2, "Both sess-old and sess-cancel should be pruned");

        let (after_db, after_wal) = pool.file_sizes();
        let after_total = after_db + after_wal;
        assert!(
            after_total < before_total && reclaimed > 0,
            "Incremental vacuum should shrink file size (before={}, after={}, reclaimed={})",
            before_total,
            after_total,
            reclaimed
        );

        let conn = pool.lock_conn();
        let old_files: i64 = conn
            .query_row("SELECT COUNT(*) FROM files WHERE session_id = 'sess-old'", [], |r| r.get(0))
            .unwrap();
        let cancel_files: i64 = conn
            .query_row("SELECT COUNT(*) FROM files WHERE session_id = 'sess-cancel'", [], |r| r.get(0))
            .unwrap();
        let new_files: i64 = conn
            .query_row("SELECT COUNT(*) FROM files WHERE session_id = 'sess-new'", [], |r| r.get(0))
            .unwrap();
        let hist_sessions: i64 = conn
            .query_row("SELECT COUNT(*) FROM scan_sessions WHERE drive_id = 'MAC'", [], |r| r.get(0))
            .unwrap();
        let cat_sums: i64 = conn
            .query_row("SELECT COUNT(*) FROM category_summaries WHERE session_id = 'sess-old'", [], |r| r.get(0))
            .unwrap();
        let snaps: i64 = conn
            .query_row("SELECT COUNT(*) FROM storage_snapshots WHERE session_id = 'sess-old'", [], |r| r.get(0))
            .unwrap();

        assert_eq!(old_files, 0);
        assert_eq!(cancel_files, 0);
        assert_eq!(new_files, 10);
        assert_eq!(hist_sessions, 3, "Historical scan session metadata must stay intact");
        assert_eq!(cat_sums, 1, "Historical category summaries must stay intact");
        assert_eq!(snaps, 1, "Historical storage snapshots must stay intact");
    }
}
