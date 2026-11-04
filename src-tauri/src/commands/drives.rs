use tauri::{command, State};
use crate::error::Result;
use crate::models::drive::{DriveDetails, DriveInfo};
use crate::services::drive_detector::DriveDetector;
use crate::storage::DbPool;

#[command]
pub async fn get_drives(db: State<'_, DbPool>) -> Result<Vec<DriveInfo>> {
    let drives = DriveDetector::detect_all();

    // Persist discovered drives in SQLite
    let conn_arc = db.conn();
    let mut conn = conn_arc.lock().unwrap();
    let _ = DriveDetector::sync_to_db(&mut conn, &drives);

    Ok(drives)
}

#[command]
pub async fn get_drive_details(drive_id: String) -> Result<DriveDetails> {
    DriveDetector::get_details(&drive_id)
}

/// Backward compatibility alias for get_drives
#[command]
pub async fn list_drives(db: State<'_, DbPool>) -> Result<Vec<DriveInfo>> {
    get_drives(db).await
}
