use tauri::{command, State};

use crate::error::Result;
use crate::models::developer::{
    DeveloperAnalysisProfile, DockerStorageInfo, GlobalToolCacheItem,
};
use crate::services::developer::DeveloperService;
use crate::storage::DbPool;

#[command]
pub async fn get_developer_storage(
    session_id: Option<String>,
    force_refresh: Option<bool>,
    db: State<'_, DbPool>,
) -> Result<DeveloperAnalysisProfile> {
    let pool = (*db).clone();
    let force = force_refresh.unwrap_or(false);
    tokio::task::spawn_blocking(move || {
        DeveloperService::get_developer_storage(&pool, session_id.as_deref(), force)
    })
    .await
    .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[command]
pub async fn get_docker_storage() -> Result<DockerStorageInfo> {
    tokio::task::spawn_blocking(DeveloperService::inspect_docker_storage)
        .await
        .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))
}

#[command]
pub async fn get_global_tool_caches() -> Result<Vec<GlobalToolCacheItem>> {
    tokio::task::spawn_blocking(DeveloperService::inspect_global_caches)
        .await
        .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))
}
