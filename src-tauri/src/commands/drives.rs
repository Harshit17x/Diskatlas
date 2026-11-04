use tauri::{command, State};
use crate::error::Result;
use crate::models::drive::DriveInfo;
use crate::services::drive_detector::DriveDetector;
use crate::storage::DbPool;

#[command]
pub async fn list_drives(db: State<'_, DbPool>) -> Result<Vec<DriveInfo>> {
    let drives = DriveDetector::detect_all();

    // Persist discovered drives in SQLite
    let conn_arc = db.conn();
    let mut conn = conn_arc.lock().unwrap();
    let _ = DriveDetector::sync_to_db(&mut conn, &drives);

    Ok(drives)
}

