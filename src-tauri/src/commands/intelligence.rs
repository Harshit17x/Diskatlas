use tauri::{command, State};
use crate::error::Result;
use crate::models::intelligence::{
    RecommendationsQuery, RecommendationsResponse, StorageIntelligenceProfile,
};
use crate::services::intelligence::IntelligenceService;
use crate::storage::DbPool;

#[command]
pub async fn get_storage_intelligence(
    session_id: Option<String>,
    db: State<'_, DbPool>,
) -> Result<StorageIntelligenceProfile> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    IntelligenceService::generate_intelligence_profile(&conn, session_id.as_deref())
}

#[command]
pub async fn get_storage_recommendations(
    query: RecommendationsQuery,
    db: State<'_, DbPool>,
) -> Result<RecommendationsResponse> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    IntelligenceService::get_recommendations(&conn, query)
}
