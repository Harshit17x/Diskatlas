use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use tauri::State;

use crate::error::Result;
use crate::models::cleanup::{
    CleanupAuditQuery, CleanupAuditResponse, CleanupPlan, ExecuteCleanupRequest,
    ExecuteCleanupResponse,
};
use crate::services::cleanup::CleanupService;

#[tauri::command]
pub async fn get_cleanup_plan(
    conn_arc: State<'_, Arc<Mutex<Connection>>>,
    session_id: String,
) -> Result<CleanupPlan> {
    let conn = conn_arc.lock().unwrap();
    CleanupService::get_cleanup_plan(&conn, &session_id)
}

#[tauri::command]
pub async fn execute_cleanup(
    conn_arc: State<'_, Arc<Mutex<Connection>>>,
    request: ExecuteCleanupRequest,
) -> Result<ExecuteCleanupResponse> {
    let conn = conn_arc.lock().unwrap();
    CleanupService::execute_cleanup(&conn, request)
}

#[tauri::command]
pub async fn get_cleanup_audit_history(
    conn_arc: State<'_, Arc<Mutex<Connection>>>,
    query: CleanupAuditQuery,
) -> Result<CleanupAuditResponse> {
    let conn = conn_arc.lock().unwrap();
    CleanupService::get_cleanup_audit_history(&conn, query)
}
