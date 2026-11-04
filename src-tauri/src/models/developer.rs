use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperProjectItem {
    pub id: String,
    pub session_id: String,
    pub name: String,
    pub root_path: String,
    pub ecosystem: String,
    pub dependency_bytes: u64,
    pub cache_bytes: u64,
    pub build_output_bytes: u64,
    pub total_bytes: u64,
    pub file_count: u64,
    pub last_modified_at: Option<i64>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperEcosystemSummary {
    pub ecosystem: String,
    pub project_count: u64,
    pub total_bytes: u64,
    pub dependency_bytes: u64,
    pub cache_bytes: u64,
    pub build_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerStorageInfo {
    pub is_available: bool,
    pub images_bytes: u64,
    pub images_count: u64,
    pub containers_bytes: u64,
    pub containers_count: u64,
    pub volumes_bytes: u64,
    pub volumes_count: u64,
    pub build_cache_bytes: u64,
    pub total_bytes: u64,
    pub status_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalToolCacheItem {
    pub tool: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeveloperAnalysisProfile {
    pub session_id: String,
    pub total_developer_bytes: u64,
    pub total_projects: u64,
    pub ecosystems: Vec<DeveloperEcosystemSummary>,
    pub projects: Vec<DeveloperProjectItem>,
    pub docker: DockerStorageInfo,
    pub global_caches: Vec<GlobalToolCacheItem>,
}
