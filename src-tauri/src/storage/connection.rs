use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use tracing::info;
use crate::error::{AppError, Result};

#[derive(Clone)]
pub struct DbPool {
    conn: Arc<Mutex<Connection>>,
    db_path: PathBuf,
}

impl DbPool {
    /// Initializes a SQLite connection at the given path (or in-memory for testing).
    /// Enforces WAL mode, foreign keys, and busy timeout for safe concurrent operations.
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
        conn.pragma_update(None, "cache_size", -64000)?; // 64 MB page cache in memory
        conn.pragma_update(None, "temp_store", "MEMORY")?; // Store temp tables & indexes in RAM
        conn.pragma_update(None, "mmap_size", 268435456)?; // 256 MB memory-mapped I/O

        info!("SQLite connected at '{}' with WAL mode enabled", path.display());

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: path.to_path_buf(),
        })
    }

    /// Creates an in-memory SQLite database instance (primarily for automated testing).
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: PathBuf::from(":memory:"),
        })
    }

    /// Access the mutex-guarded SQLite connection
    pub fn conn(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Count all user tables present in SQLite master
    pub fn count_tables(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
        )?;
        let count: usize = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }
}

