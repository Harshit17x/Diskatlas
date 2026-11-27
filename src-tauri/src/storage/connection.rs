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
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")?;
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

        let (needs_dirs_unique, needs_orphans, needs_summaries) = self.with_read_conn(|conn| {
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
            ))
        })?;

        if !needs_dirs_unique && !needs_orphans && !needs_summaries {
            return Ok(());
        }

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

        info!("Deferred database startup maintenance complete");
        Ok(())
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
            assert_eq!(version, 11);
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
                 WHERE key IN ('maintenance:dirs_unique_pending', 'maintenance:orphans_pending', 'maintenance:summaries_pending')",
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
            assert_eq!(Migrator::run(&mut conn).unwrap(), 11);
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
}
