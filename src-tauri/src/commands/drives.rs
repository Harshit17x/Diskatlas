use tauri::{command, State};
use crate::error::Result;
use crate::models::drive::{DriveDetails, DriveInfo};
use crate::services::drive_detector::DriveDetector;
use crate::storage::DbPool;

#[command]
pub async fn get_drives(db: State<'_, DbPool>) -> Result<Vec<DriveInfo>> {
    let drives = DriveDetector::detect_all();

    // Persist discovered drives in SQLite asynchronously in background so UI is NEVER blocked
    let db_pool = (*db).clone();
    let drives_clone = drives.clone();
    tokio::task::spawn_blocking(move || {
        let conn_arc = db_pool.conn();
        let guard = conn_arc.lock();
        if let Ok(mut conn) = guard {
            let _ = DriveDetector::sync_to_db(&mut conn, &drives_clone);
        }
    });

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
