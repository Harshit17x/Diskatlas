use tauri::State;

use crate::error::{AppError, Result};
use crate::models::cleanup::{
    CleanupAuditQuery, CleanupAuditResponse, CleanupPlan, ExecuteCleanupRequest,
    ExecuteCleanupResponse,
};
use crate::services::cleanup::CleanupService;
use crate::storage::DbPool;

#[tauri::command]
pub async fn get_cleanup_plan(
    db: State<'_, DbPool>,
    session_id: Option<String>,
) -> Result<CleanupPlan> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.lock_conn();
        CleanupService::get_cleanup_plan(&conn, session_id.as_deref())
    })
    .await
    .map_err(|e| AppError::System(format!("Cleanup plan task failed: {}", e)))?
}

#[tauri::command]
pub async fn execute_cleanup(
    db: State<'_, DbPool>,
    request: ExecuteCleanupRequest,
) -> Result<ExecuteCleanupResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.lock_conn();
        CleanupService::execute_cleanup(&conn, request)
    })
    .await
    .map_err(|e| AppError::System(format!("Execute cleanup task failed: {}", e)))?
}

#[tauri::command]
pub async fn get_cleanup_audit_history(
    db: State<'_, DbPool>,
    query: CleanupAuditQuery,
) -> Result<CleanupAuditResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| CleanupService::get_cleanup_audit_history(conn, query))
    })
    .await
    .map_err(|e| AppError::System(format!("Cleanup audit task failed: {}", e)))?
}
