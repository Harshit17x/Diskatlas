use tauri::{command, State};
use crate::error::{AppError, Result};
use crate::models::intelligence::{
    RecommendationsQuery, RecommendationsResponse, StorageIntelligenceProfile,
};
use crate::services::intelligence::IntelligenceService;
use crate::storage::DbPool;

#[command]
pub async fn get_storage_intelligence(
    session_id: Option<String>,
    force_refresh: Option<bool>,
    db: State<'_, DbPool>,
) -> Result<StorageIntelligenceProfile> {
    let pool = (*db).clone();
    let refresh = force_refresh.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        IntelligenceService::get_or_generate_profile(&pool, session_id.as_deref(), refresh)
    })
    .await
    .map_err(|e| AppError::System(format!("Intelligence task failed: {}", e)))?
}

#[command]
pub async fn get_storage_recommendations(
    query: RecommendationsQuery,
    db: State<'_, DbPool>,
) -> Result<RecommendationsResponse> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| IntelligenceService::get_recommendations(conn, query))
    })
    .await
    .map_err(|e| AppError::System(format!("Recommendations task failed: {}", e)))?
}

