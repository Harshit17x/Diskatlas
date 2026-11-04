use tauri::{command, State};
use crate::error::Result;
use crate::models::visualization::{HierarchicalNode, HierarchicalQuery, StorageAnalyticsData};
use crate::services::visualization::VisualizationService;
use crate::storage::DbPool;

#[command]
pub async fn get_hierarchical_tree(
    query: HierarchicalQuery,
    db: State<'_, DbPool>,
) -> Result<HierarchicalNode> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    VisualizationService::get_hierarchical_tree(&conn, query)
}

#[command]
pub async fn get_storage_analytics(
    session_id: Option<String>,
    db: State<'_, DbPool>,
) -> Result<StorageAnalyticsData> {
    let conn_arc = db.conn();
    let conn = conn_arc.lock().unwrap();
    VisualizationService::get_storage_analytics(&conn, session_id.as_deref())
}

