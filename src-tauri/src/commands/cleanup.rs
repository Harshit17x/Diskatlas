use tauri::State;

use crate::error::Result;
use crate::models::cleanup::{
    CleanupAuditQuery, CleanupAuditResponse, CleanupPlan, ExecuteCleanupRequest,
    ExecuteCleanupResponse,
};
use crate::services::cleanup::CleanupService;
use crate::storage::DbPool;

#[tauri::command]
pub async fn get_cleanup_plan(
    db: State<'_, DbPool>,
    session_id: String,
) -> Result<CleanupPlan> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    CleanupService::get_cleanup_plan(&conn, &session_id)
}

#[tauri::command]
pub async fn execute_cleanup(
    db: State<'_, DbPool>,
    request: ExecuteCleanupRequest,
) -> Result<ExecuteCleanupResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    CleanupService::execute_cleanup(&conn, request)
}

#[tauri::command]
pub async fn get_cleanup_audit_history(
    db: State<'_, DbPool>,
    query: CleanupAuditQuery,
) -> Result<CleanupAuditResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    CleanupService::get_cleanup_audit_history(&conn, query)
}
