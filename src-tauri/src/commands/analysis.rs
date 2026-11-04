use tauri::{command, State};
use crate::error::Result;
use crate::models::analysis::*;
use crate::services::analysis::AnalysisService;
use crate::storage::DbPool;

#[command]
pub async fn get_large_files(
    query: LargeFilesQuery,
    db: State<'_, DbPool>,
) -> Result<LargeFilesResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    AnalysisService::get_large_files(&conn, query)
}

#[command]
pub async fn get_hidden_files(
    query: HiddenFilesQuery,
    db: State<'_, DbPool>,
) -> Result<HiddenFilesResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    AnalysisService::get_hidden_files(&conn, query)
}

#[command]
pub async fn find_duplicates(
    query: DuplicateSearchQuery,
    db: State<'_, DbPool>,
) -> Result<DuplicateSearchResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    AnalysisService::find_duplicates(&conn, query)
}

#[command]
pub async fn get_forgotten_files(
    query: ForgottenFilesQuery,
    db: State<'_, DbPool>,
) -> Result<ForgottenFilesResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    AnalysisService::get_forgotten_files(&conn, query)
}

#[command]
pub async fn get_unknown_storage(
    session_id: Option<String>,
    db: State<'_, DbPool>,
) -> Result<UnknownStorageResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    AnalysisService::get_unknown_storage(&conn, session_id.as_deref())
}
