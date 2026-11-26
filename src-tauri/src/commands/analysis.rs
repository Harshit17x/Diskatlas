use tauri::{command, State};
use crate::error::{AppError, Result};
use crate::models::analysis::*;
use crate::services::analysis::AnalysisService;
use crate::storage::DbPool;

#[command]
pub async fn get_large_files(
    query: LargeFilesQuery,
    db: State<'_, DbPool>,
) -> Result<LargeFilesResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| AnalysisService::get_large_files(conn, query))
    })
    .await
    .map_err(|e| AppError::System(format!("Large files task failed: {}", e)))?
}

#[command]
pub async fn get_hidden_files(
    query: HiddenFilesQuery,
    db: State<'_, DbPool>,
) -> Result<HiddenFilesResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| AnalysisService::get_hidden_files(conn, query))
    })
    .await
    .map_err(|e| AppError::System(format!("Hidden files task failed: {}", e)))?
}

#[command]
pub async fn find_duplicates(
    query: DuplicateSearchQuery,
    db: State<'_, DbPool>,
) -> Result<DuplicateSearchResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || AnalysisService::find_duplicates_pool(&pool, query))
        .await
        .map_err(|e| AppError::System(format!("Duplicate search task failed: {}", e)))?
}

#[command]
pub async fn get_forgotten_files(
    query: ForgottenFilesQuery,
    db: State<'_, DbPool>,
) -> Result<ForgottenFilesResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| AnalysisService::get_forgotten_files(conn, query))
    })
    .await
    .map_err(|e| AppError::System(format!("Forgotten files task failed: {}", e)))?
}

#[command]
pub async fn get_unknown_storage(
    session_id: Option<String>,
    db: State<'_, DbPool>,
) -> Result<UnknownStorageResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| AnalysisService::get_unknown_storage(conn, session_id.as_deref()))
    })
    .await
    .map_err(|e| AppError::System(format!("Unknown storage task failed: {}", e)))?
}
