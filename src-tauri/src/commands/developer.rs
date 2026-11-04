use tauri::{command, State};

use crate::error::Result;
use crate::models::developer::{
    DeveloperAnalysisProfile, DockerStorageInfo, GlobalToolCacheItem,
};
use crate::services::developer::DeveloperService;
use crate::storage::DbPool;

#[command]
pub async fn get_developer_storage(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<DeveloperAnalysisProfile> {
    DeveloperService::analyze_developer_storage(&db, &session_id)
}

#[command]
pub async fn get_docker_storage() -> Result<DockerStorageInfo> {
    Ok(DeveloperService::inspect_docker_storage())
}

#[command]
pub async fn get_global_tool_caches() -> Result<Vec<GlobalToolCacheItem>> {
    Ok(DeveloperService::inspect_global_caches())
}
