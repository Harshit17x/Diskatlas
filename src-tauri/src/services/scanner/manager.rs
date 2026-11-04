use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tracing::info;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::scan::{ScanFileItem, ScanFilePage, ScanProgress, ScanStatus};
use crate::services::scanner::engine::{
    ScanEngine, STATUS_CANCELLED, STATUS_PAUSED, STATUS_RUNNING,
};
use crate::services::scanner::models::ScanOptions;
use crate::storage::DbPool;

pub struct ActiveScanHandle {
    pub session_id: String,
    pub status: Arc<AtomicU8>,
    pub pause_notify: Arc<tokio::sync::Notify>,
    pub progress: Arc<Mutex<ScanProgress>>,
}

#[derive(Clone)]
pub struct ScanManager {
    sessions: Arc<Mutex<HashMap<String, Arc<ActiveScanHandle>>>>,
}

impl Default for ScanManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Marks any orphaned running/paused scan sessions from previous application crashes as interrupted.
    pub fn recover_interrupted_scans(&self, db_pool: &DbPool) -> Result<usize> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();
        let count = conn.execute(
            "UPDATE scan_sessions
             SET status = 'interrupted', completed_at = CURRENT_TIMESTAMP
             WHERE status IN ('running', 'paused')",
            [],
        )?;
        if count > 0 {
            info!("Recovered {} orphaned/interrupted scan session(s)", count);
        }
        Ok(count)
    }

    /// Starts a background recursive filesystem scan.
    pub fn start_scan(
        &self,
        path: PathBuf,
        drive_id: String,
        exclusions: Option<Vec<String>>,
        db_pool: DbPool,
        app_handle: Option<AppHandle>,
    ) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let status = Arc::new(AtomicU8::new(STATUS_RUNNING));
        let pause_notify = Arc::new(tokio::sync::Notify::new());

        let initial_progress = ScanProgress {
            session_id: session_id.clone(),
            drive_id: drive_id.clone(),
            root_path: path.to_string_lossy().to_string(),
            status: ScanStatus::Running,
            files_scanned: 0,
            dirs_scanned: 0,
            bytes_scanned: 0,
            error_count: 0,
            current_path: path.to_string_lossy().to_string(),
            elapsed_millis: 0,
        };
        let progress = Arc::new(Mutex::new(initial_progress));

        let handle = Arc::new(ActiveScanHandle {
            session_id: session_id.clone(),
            status: Arc::clone(&status),
            pause_notify: Arc::clone(&pause_notify),
            progress: Arc::clone(&progress),
        });

        {
            let mut map = self.sessions.lock().unwrap();
            map.insert(session_id.clone(), handle);
        }

        let options = ScanOptions {
            session_id: session_id.clone(),
            drive_id,
            root_path: path,
            exclusions: exclusions.unwrap_or_default(),
            batch_size: 500,
        };

        // Spawn background async task
        tokio::spawn(async move {
            let _ = ScanEngine::run_scan(
                options,
                status,
                pause_notify,
                progress,
                db_pool,
                app_handle,
            )
            .await;
        });

        Ok(session_id)
    }

    /// Pauses an active scan.
    pub fn pause_scan(&self, session_id: &str) -> Result<()> {
        let map = self.sessions.lock().unwrap();
        let handle = map
            .get(session_id)
            .ok_or_else(|| AppError::System(format!("No active scan session found for id '{}'", session_id)))?;

        handle.status.store(STATUS_PAUSED, Ordering::SeqCst);
        {
            let mut p = handle.progress.lock().unwrap();
            p.status = ScanStatus::Paused;
        }
        info!("Scan session '{}' requested PAUSE", session_id);
        Ok(())
    }

    /// Resumes a paused scan.
    pub fn resume_scan(&self, session_id: &str) -> Result<()> {
        let map = self.sessions.lock().unwrap();
        let handle = map
            .get(session_id)
            .ok_or_else(|| AppError::System(format!("No active scan session found for id '{}'", session_id)))?;

        handle.status.store(STATUS_RUNNING, Ordering::SeqCst);
        {
            let mut p = handle.progress.lock().unwrap();
            p.status = ScanStatus::Running;
        }
        handle.pause_notify.notify_waiters();
        info!("Scan session '{}' requested RESUME", session_id);
        Ok(())
    }

    /// Cancels an active scan.
    pub fn cancel_scan(&self, session_id: &str) -> Result<()> {
        let map = self.sessions.lock().unwrap();
        let handle = map
            .get(session_id)
            .ok_or_else(|| AppError::System(format!("No active scan session found for id '{}'", session_id)))?;

        handle.status.store(STATUS_CANCELLED, Ordering::SeqCst);
        {
            let mut p = handle.progress.lock().unwrap();
            p.status = ScanStatus::Cancelled;
        }
        // Also wake up if it was paused so it can observe the cancel flag
        handle.pause_notify.notify_waiters();
        info!("Scan session '{}' requested CANCEL", session_id);
        Ok(())
    }

    /// Retrieves current progress for a scan session.
    pub fn get_progress(&self, session_id: &str, db_pool: &DbPool) -> Result<ScanProgress> {
        // First check in-memory active sessions
        {
            let map = self.sessions.lock().unwrap();
            if let Some(handle) = map.get(session_id) {
                let p = handle.progress.lock().unwrap();
                return Ok(p.clone());
            }
        }

        // Otherwise query historical session from SQLite
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT drive_id, root_path, status, total_files, total_directories, total_bytes, error_count
             FROM scan_sessions WHERE id = ?1",
        )?;

        let progress = stmt.query_row([session_id], |row| {
            let drive_id: String = row.get(0)?;
            let root_path: String = row.get(1)?;
            let status_str: String = row.get(2)?;
            let files: i64 = row.get(3)?;
            let dirs: i64 = row.get(4)?;
            let bytes: i64 = row.get(5)?;
            let errors: i64 = row.get(6)?;

            let status = match status_str.as_str() {
                "running" => ScanStatus::Running,
                "paused" => ScanStatus::Paused,
                "completed" => ScanStatus::Completed,
                "completed_with_errors" => ScanStatus::CompletedWithErrors,
                "cancelled" => ScanStatus::Cancelled,
                _ => ScanStatus::Failed,
            };

            Ok(ScanProgress {
                session_id: session_id.to_string(),
                drive_id,
                root_path,
                status,
                files_scanned: files as u64,
                dirs_scanned: dirs as u64,
                bytes_scanned: bytes as u64,
                error_count: errors as u64,
                current_path: String::new(),
                elapsed_millis: 0,
            })
        })?;

        Ok(progress)
    }

    /// Paginated query for scanned files in a session.
    pub fn get_files(
        &self,
        session_id: &str,
        page: u32,
        page_size: u32,
        parent_path: Option<String>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        db_pool: &DbPool,
    ) -> Result<ScanFilePage> {
        let conn_arc = db_pool.conn();
        let conn = conn_arc.lock().unwrap();

        let offset = page.saturating_sub(1) * page_size;

        // Count total matching files
        let total_count: i64 = if let Some(ref parent) = parent_path {
            conn.query_row(
                "SELECT count(*) FROM files WHERE session_id = ?1 AND parent_path = ?2",
                (session_id, parent),
                |row| row.get(0),
            )?
        } else {
            conn.query_row(
                "SELECT count(*) FROM files WHERE session_id = ?1",
                [session_id],
                |row| row.get(0),
            )?
        };

        let order_col = match sort_by.as_deref() {
            Some("size") => "size_bytes",
            Some("name") => "name",
            Some("modified") => "modified_at",
            _ => "size_bytes",
        };
        let direction = if sort_desc.unwrap_or(true) { "DESC" } else { "ASC" };

        let query = if parent_path.is_some() {
            format!(
                "SELECT id, session_id, path, name, extension, parent_path, size_bytes,
                        allocated_bytes, created_at, modified_at, is_hidden, is_system, is_symlink
                 FROM files WHERE session_id = ?1 AND parent_path = ?2
                 ORDER BY {} {} LIMIT ?3 OFFSET ?4",
                order_col, direction
            )
        } else {
            format!(
                "SELECT id, session_id, path, name, extension, parent_path, size_bytes,
                        allocated_bytes, created_at, modified_at, is_hidden, is_system, is_symlink
                 FROM files WHERE session_id = ?1
                 ORDER BY {} {} LIMIT ?2 OFFSET ?3",
                order_col, direction
            )
        };

        let mut stmt = conn.prepare(&query)?;

        let items: std::result::Result<Vec<ScanFileItem>, rusqlite::Error> = if let Some(ref parent) = parent_path {
            stmt.query_map((session_id, parent, page_size, offset), Self::map_file_row)?
                .collect()
        } else {
            stmt.query_map((session_id, page_size, offset), Self::map_file_row)?
                .collect()
        };

        let items = items?;
        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as u32;

        Ok(ScanFilePage {
            items,
            total_count: total_count as u64,
            page,
            page_size,
            total_pages: total_pages.max(1),
        })
    }

    fn map_file_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanFileItem> {
        let size: i64 = row.get(6)?;
        let allocated: Option<i64> = row.get(7)?;
        let is_hidden: i32 = row.get(10)?;
        let is_system: i32 = row.get(11)?;
        let is_symlink: i32 = row.get(12)?;

        Ok(ScanFileItem {
            id: row.get(0)?,
            session_id: row.get(1)?,
            path: row.get(2)?,
            name: row.get(3)?,
            extension: row.get(4)?,
            parent_path: row.get(5)?,
            size_bytes: size as u64,
            allocated_bytes: allocated.map(|b| b as u64),
            created_at: row.get(8)?,
            modified_at: row.get(9)?,
            is_hidden: is_hidden != 0,
            is_system: is_system != 0,
            is_symlink: is_symlink != 0,
        })
    }
}

