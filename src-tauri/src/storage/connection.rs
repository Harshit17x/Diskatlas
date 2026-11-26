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
}
