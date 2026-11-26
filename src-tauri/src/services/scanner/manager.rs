use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use tauri::AppHandle;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::models::scan::{ScanFileItem, ScanFilePage, ScanProgress, ScanStatus};
use crate::services::scanner::engine::{
    ScanEngine, STATUS_CANCELLED, STATUS_FAILED, STATUS_PAUSED, STATUS_RUNNING,
};
use crate::services::scanner::models::ScanOptions;
use crate::storage::{resolve_session_id, DbPool};

pub struct ActiveScanHandle {
    pub session_id: String,
    pub start_time: std::time::Instant,
    pub status: Arc<AtomicU8>,
    pub pause_signal: Arc<(Mutex<()>, Condvar)>,
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

struct SessionCleanupGuard {
    sessions: Arc<Mutex<HashMap<String, Arc<ActiveScanHandle>>>>,
    session_id: String,
}

impl Drop for SessionCleanupGuard {
    fn drop(&mut self) {
        let mut map = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&self.session_id);
    }
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn lock_sessions(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<ActiveScanHandle>>> {
        self.sessions.lock().unwrap_or_else(|e| {
            warn!("Recovering from poisoned ScanManager sessions mutex");
            e.into_inner()
        })
    }

    /// Marks any orphaned running/paused scan sessions from previous application crashes as interrupted.
    pub fn recover_interrupted_scans(&self, db_pool: &DbPool) -> Result<usize> {
        let conn = db_pool.lock_conn();
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
        let pause_signal = Arc::new((Mutex::new(()), Condvar::new()));

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
            current_file: None,
            recent_files: Vec::new(),
            elapsed_millis: 0,
        };
        let progress = Arc::new(Mutex::new(initial_progress));

        let start_time = std::time::Instant::now();
        let handle = Arc::new(ActiveScanHandle {
            session_id: session_id.clone(),
            start_time,
            status: Arc::clone(&status),
            pause_signal: Arc::clone(&pause_signal),
            progress: Arc::clone(&progress),
        });

        {
            let mut map = self.lock_sessions();
            map.insert(session_id.clone(), handle);
        }

        let options = ScanOptions {
            session_id: session_id.clone(),
            drive_id,
            root_path: path,
            exclusions: exclusions.unwrap_or_default(),
            batch_size: 2500,
        };

        let sid_err = session_id.clone();
        let status_err = Arc::clone(&status);
        let progress_err = Arc::clone(&progress);
        let handle_err = app_handle.clone();
        let sessions_ref = Arc::clone(&self.sessions);

        let spawn_res = std::thread::Builder::new()
            .name(format!("diskatlas-scan-{}", &session_id[..8]))
            .spawn(move || {
                let _guard = SessionCleanupGuard {
                    sessions: sessions_ref,
                    session_id: sid_err.clone(),
                };

                let db_for_err = db_pool.clone();
                let res = ScanEngine::run_scan(
                    options,
                    status,
                    pause_signal,
                    progress,
                    db_pool,
                    app_handle,
                );

                if let Err(err) = res {
                    tracing::error!("ScanEngine error on session '{}': {:?}", sid_err, err);
                    status_err.store(STATUS_FAILED, Ordering::SeqCst);
                    {
                        let mut p = progress_err.lock().unwrap_or_else(|e| e.into_inner());
                        p.status = ScanStatus::Failed;
                    }
                    // Persist failed status in SQLite so the session is never left 'running'
                    {
                        let conn = db_for_err.lock_conn();
                        let _ = conn.execute(
                            "UPDATE scan_sessions
                             SET status = 'failed', completed_at = CURRENT_TIMESTAMP
                             WHERE id = ?1",
                            [&sid_err],
                        );
                    }
                    if let Some(ref handle) = handle_err {
                        use tauri::Emitter;
                        handle.emit("scan://error", &err.to_string()).ok();
                    }
                }
            });

        if let Err(e) = spawn_res {
            self.lock_sessions().remove(&session_id);
            return Err(AppError::System(format!("Failed to spawn scan thread: {}", e)));
        }

        Ok(session_id)
    }

    /// Pauses an active scan.
    pub fn pause_scan(&self, session_id: &str) -> Result<()> {
        let map = self.lock_sessions();
        let handle = map
            .get(session_id)
            .ok_or_else(|| AppError::System(format!("No active scan session found for id '{}'", session_id)))?;

        handle.status.store(STATUS_PAUSED, Ordering::SeqCst);
        {
            let mut p = handle.progress.lock().unwrap_or_else(|e| e.into_inner());
            p.status = ScanStatus::Paused;
        }
        info!("Scan session '{}' requested PAUSE", session_id);
        Ok(())
    }

    /// Resumes a paused scan.
    pub fn resume_scan(&self, session_id: &str) -> Result<()> {
        let map = self.lock_sessions();
        let handle = map
            .get(session_id)
            .ok_or_else(|| AppError::System(format!("No active scan session found for id '{}'", session_id)))?;

        handle.status.store(STATUS_RUNNING, Ordering::SeqCst);
        {
            let mut p = handle.progress.lock().unwrap_or_else(|e| e.into_inner());
            p.status = ScanStatus::Running;
        }
        handle.pause_signal.1.notify_all();
        info!("Scan session '{}' requested RESUME", session_id);
        Ok(())
    }

    /// Cancels an active scan.
    pub fn cancel_scan(&self, session_id: &str) -> Result<()> {
        let map = self.lock_sessions();
        let handle = map
            .get(session_id)
            .ok_or_else(|| AppError::System(format!("No active scan session found for id '{}'", session_id)))?;

        handle.status.store(STATUS_CANCELLED, Ordering::SeqCst);
        {
            let mut p = handle.progress.lock().unwrap_or_else(|e| e.into_inner());
            p.status = ScanStatus::Cancelled;
        }
        handle.pause_signal.1.notify_all();
        info!("Scan session '{}' requested CANCEL", session_id);
        Ok(())
    }

    /// Retrieves current progress for a scan session.
    pub fn get_progress(&self, session_id: &str, db_pool: &DbPool) -> Result<ScanProgress> {
        {
            let map = self.lock_sessions();
            if let Some(handle) = map.get(session_id) {
                let mut p = handle.progress.lock().unwrap_or_else(|e| e.into_inner()).clone();
                if p.status == ScanStatus::Running {
                    p.elapsed_millis = handle.start_time.elapsed().as_millis() as u64;
                }
                return Ok(p);
            }
        }

        db_pool.with_read_conn(|conn| {
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

                Ok(ScanProgress {
                    session_id: session_id.to_string(),
                    drive_id,
                    root_path,
                    status: ScanStatus::from_str(&status_str),
                    files_scanned: files as u64,
                    dirs_scanned: dirs as u64,
                    bytes_scanned: bytes as u64,
                    error_count: errors as u64,
                    current_path: String::new(),
                    current_file: None,
                    recent_files: Vec::new(),
                    elapsed_millis: 0,
                })
            })?;

            Ok(progress)
        })
    }

    /// Paginated query for scanned files in a session (resolves latest completed session if `session_id` is None).
    #[allow(clippy::too_many_arguments)]
    pub fn get_files(
        &self,
        session_id: Option<&str>,
        page: u32,
        page_size: u32,
        parent_path: Option<String>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        db_pool: &DbPool,
    ) -> Result<ScanFilePage> {
        db_pool.with_read_conn(|conn| {
            let sid = resolve_session_id(conn, session_id)?;
            let safe_page = page.max(1);
            let safe_size = page_size.clamp(1, 200);
            let offset = (safe_page - 1) * safe_size;

            let clean_parent = parent_path.and_then(|p| {
                let t = p.trim().to_string();
                if t.is_empty() { None } else { Some(t) }
            });

            // Note: when viewing root `/` or a session root, include direct files if any exist;
            // if user selected root and no direct files exist on root (e.g. macOS firmlink roots),
            // let caller still see either direct files or all session files when parent_path is omitted.
            let total_count: i64 = if let Some(ref parent) = clean_parent {
                conn.query_row(
                    "SELECT count(*) FROM files WHERE session_id = ?1 AND parent_path = ?2",
                    (&sid, parent),
                    |row| row.get(0),
                )?
            } else {
                conn.query_row(
                    "SELECT count(*) FROM files WHERE session_id = ?1",
                    [&sid],
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

            let query = if clean_parent.is_some() {
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

            let items: std::result::Result<Vec<ScanFileItem>, rusqlite::Error> = if let Some(ref parent) = clean_parent {
                stmt.query_map((&sid, parent, safe_size, offset), Self::map_file_row)?
                    .collect()
            } else {
                stmt.query_map((&sid, safe_size, offset), Self::map_file_row)?
                    .collect()
            };

            let items = items?;
            let total_pages = ((total_count as f64) / (safe_size as f64)).ceil() as u32;

            Ok(ScanFilePage {
                items,
                total_count: total_count as u64,
                page: safe_page,
                page_size: safe_size,
                total_pages: total_pages.max(1),
            })
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
