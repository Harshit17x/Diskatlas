use tauri::{command, State};
use crate::error::{AppError, Result};
use crate::models::visualization::{HierarchicalNode, HierarchicalQuery, StorageAnalyticsData};
use crate::services::visualization::VisualizationService;
use crate::storage::DbPool;

#[command]
pub async fn get_hierarchical_tree(
    query: HierarchicalQuery,
    db: State<'_, DbPool>,
) -> Result<HierarchicalNode> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| VisualizationService::get_hierarchical_tree(conn, query))
    })
    .await
    .map_err(|e| AppError::System(format!("Visualization task failed: {}", e)))?
}

#[command]
pub async fn get_storage_analytics(
    session_id: Option<String>,
    db: State<'_, DbPool>,
) -> Result<StorageAnalyticsData> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        pool.with_read_conn(|conn| {
            VisualizationService::get_storage_analytics(conn, session_id.as_deref())
        })
    })
    .await
    .map_err(|e| AppError::System(format!("Analytics task failed: {}", e)))?
}

