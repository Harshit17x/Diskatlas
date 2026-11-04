use tauri::{command, State};

use crate::error::Result;
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
    let p = page.unwrap_or(1);
    let ps = page_size.unwrap_or(20);
    HistoryService::get_scan_sessions(&db, p, ps, drive_id, status)
}

#[command]
pub async fn get_session_details(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<SessionDetails> {
    HistoryService::get_session_details(&db, &session_id)
}

#[command]
pub async fn delete_scan_session(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<()> {
    HistoryService::delete_scan_session(&db, &session_id)
}

#[command]
pub async fn get_storage_snapshots(
    drive_id: Option<String>,
    limit: Option<u32>,
    db: State<'_, DbPool>,
) -> Result<Vec<StorageSnapshotRecord>> {
    let l = limit.unwrap_or(20);
    HistoryService::get_storage_snapshots(&db, drive_id, l)
}

#[command]
pub async fn create_manual_snapshot(
    drive_id: String,
    label: String,
    db: State<'_, DbPool>,
) -> Result<StorageSnapshotRecord> {
    HistoryService::create_manual_snapshot(&db, &drive_id, &label)
}

#[command]
pub async fn compare_scans(
    baseline_session_id: String,
    target_session_id: String,
    db: State<'_, DbPool>,
) -> Result<ScanComparisonResult> {
    HistoryService::compare_scans(&db, &baseline_session_id, &target_session_id)
}

