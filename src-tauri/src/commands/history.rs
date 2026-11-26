use tauri::{command, State};

use crate::error::{AppError, Result};
use crate::models::history::{
    ScanComparisonResult, ScanSessionPage, SessionDetails, StorageSnapshotRecord,
};
use crate::services::history::HistoryService;
use crate::storage::DbPool;

#[command]
pub async fn get_scan_sessions(
    page: Option<u32>,
    page_size: Option<u32>,
    drive_id: Option<String>,
    status: Option<String>,
    db: State<'_, DbPool>,
) -> Result<ScanSessionPage> {
    let pool = (*db).clone();
    let p = page.unwrap_or(1);
    let ps = page_size.unwrap_or(20);
    tokio::task::spawn_blocking(move || {
        HistoryService::get_scan_sessions(&pool, p, ps, drive_id, status)
    })
    .await
    .map_err(|e| AppError::System(format!("Scan sessions task failed: {}", e)))?
}

#[command]
pub async fn get_session_details(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<SessionDetails> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || HistoryService::get_session_details(&pool, &session_id))
        .await
        .map_err(|e| AppError::System(format!("Session details task failed: {}", e)))?
}

#[command]
pub async fn delete_scan_session(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<()> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || HistoryService::delete_scan_session(&pool, &session_id))
        .await
        .map_err(|e| AppError::System(format!("Delete session task failed: {}", e)))?
}

#[command]
pub async fn get_storage_snapshots(
    drive_id: Option<String>,
    limit: Option<u32>,
    db: State<'_, DbPool>,
) -> Result<Vec<StorageSnapshotRecord>> {
    let pool = (*db).clone();
    let l = limit.unwrap_or(20);
    tokio::task::spawn_blocking(move || HistoryService::get_storage_snapshots(&pool, drive_id, l))
        .await
        .map_err(|e| AppError::System(format!("Snapshots task failed: {}", e)))?
}

#[command]
pub async fn create_manual_snapshot(
    drive_id: String,
    label: String,
    db: State<'_, DbPool>,
) -> Result<StorageSnapshotRecord> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        HistoryService::create_manual_snapshot(&pool, &drive_id, &label)
    })
    .await
    .map_err(|e| AppError::System(format!("Create snapshot task failed: {}", e)))?
}

#[command]
pub async fn compare_scans(
    baseline_session_id: String,
    target_session_id: String,
    db: State<'_, DbPool>,
) -> Result<ScanComparisonResult> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        HistoryService::compare_scans(&pool, &baseline_session_id, &target_session_id)
    })
    .await
    .map_err(|e| AppError::System(format!("Compare scans task failed: {}", e)))?
}

