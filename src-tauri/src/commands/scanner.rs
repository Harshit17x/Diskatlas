use std::path::PathBuf;
use tauri::{command, AppHandle, State};
use tracing::{debug, info};

use crate::error::Result;
use crate::models::scan::{ScanFilePage, ScanProgress};
use crate::services::scanner::ScanManager;
use crate::storage::DbPool;

#[command]
pub async fn start_scan(
    path: String,
    drive_id: String,
    exclusions: Option<Vec<String>>,
    manager: State<'_, ScanManager>,
    db: State<'_, DbPool>,
    app_handle: AppHandle,
) -> Result<String> {
    info!("[IPC] start_scan invoked: path='{}', drive_id='{}'", path, drive_id);
    let root_path = if path == "~" {
        std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/"))
    } else if path.starts_with("~/") || path.starts_with("~\\") {
        if let Ok(home) = std::env::var("HOME") {
            std::path::PathBuf::from(home).join(&path[2..])
        } else {
            std::path::PathBuf::from(&path)
        }
    } else {
        std::path::PathBuf::from(&path)
    };
    let session_id = manager.start_scan(root_path, drive_id, exclusions, (*db).clone(), Some(app_handle))?;
    info!("[IPC] start_scan session created: '{}'", session_id);
    Ok(session_id)
}

#[command]
pub async fn pause_scan(session_id: String, manager: State<'_, ScanManager>) -> Result<()> {
    info!("[IPC] pause_scan invoked for session '{}'", session_id);
    manager.pause_scan(&session_id)
}

#[command]
pub async fn resume_scan(session_id: String, manager: State<'_, ScanManager>) -> Result<()> {
    info!("[IPC] resume_scan invoked for session '{}'", session_id);
    manager.resume_scan(&session_id)
}

#[command]
pub async fn cancel_scan(session_id: String, manager: State<'_, ScanManager>) -> Result<()> {
    info!("[IPC] cancel_scan invoked for session '{}'", session_id);
    manager.cancel_scan(&session_id)
}

#[command]
pub async fn get_scan_progress(
    session_id: String,
    manager: State<'_, ScanManager>,
    db: State<'_, DbPool>,
) -> Result<ScanProgress> {
    let p = manager.get_progress(&session_id, &db)?;
    debug!(
        "[IPC] get_scan_progress for '{}': status={:?}, files={}, dirs={}",
        session_id, p.status, p.files_scanned, p.dirs_scanned
    );
    Ok(p)
}

#[command]
pub async fn get_scan_files(
    session_id: String,
    page: Option<u32>,
    page_size: Option<u32>,
    parent_path: Option<String>,
    sort_by: Option<String>,
    sort_desc: Option<bool>,
    manager: State<'_, ScanManager>,
    db: State<'_, DbPool>,
) -> Result<ScanFilePage> {
    let p = page.unwrap_or(1);
    let ps = page_size.unwrap_or(50);
    manager.get_files(&session_id, p, ps, parent_path, sort_by, sort_desc, &db)
}

