use std::path::PathBuf;
use tauri::{command, AppHandle, State};

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
    let root_path = PathBuf::from(&path);
    manager.start_scan(root_path, drive_id, exclusions, (*db).clone(), Some(app_handle))
}

#[command]
pub async fn pause_scan(session_id: String, manager: State<'_, ScanManager>) -> Result<()> {
    manager.pause_scan(&session_id)
}

#[command]
pub async fn resume_scan(session_id: String, manager: State<'_, ScanManager>) -> Result<()> {
    manager.resume_scan(&session_id)
}

#[command]
pub async fn cancel_scan(session_id: String, manager: State<'_, ScanManager>) -> Result<()> {
    manager.cancel_scan(&session_id)
}

#[command]
pub async fn get_scan_progress(
    session_id: String,
    manager: State<'_, ScanManager>,
    db: State<'_, DbPool>,
) -> Result<ScanProgress> {
    manager.get_progress(&session_id, &db)
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

