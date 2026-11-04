use serde::{Deserialize, Serialize};
use crate::models::developer::DeveloperEcosystemSummary;
use crate::models::history::StorageSnapshotRecord;

/// A node in the hierarchical filesystem tree for Treemap and Sunburst rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchicalNode {
    pub name: String,
    pub path: String,
    pub value: i64,
    pub category: Option<String>,
    pub is_directory: bool,
    pub file_count: Option<i64>,
    pub children: Option<Vec<HierarchicalNode>>,
}

/// Query parameters for hierarchical tree aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HierarchicalQuery {
    pub session_id: Option<String>,
    pub root_path: Option<String>,
    pub max_depth: Option<u32>,
    pub min_bytes_threshold: Option<i64>,
}

/// Extension / file type distribution item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionDistItem {
    pub extension: String,
    pub file_count: i64,
    pub total_bytes: i64,
    pub percentage: f64,
}

/// Application storage distribution item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStorageItem {
    pub app_name: String,
    pub file_count: i64,
    pub total_bytes: i64,
    pub percentage: f64,
}

/// Top largest directories item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargestDirItem {
    pub path: String,
    pub name: String,
    pub total_bytes: i64,
    pub file_count: i64,
    pub dir_count: i64,
    pub percentage: f64,
}

/// Category distribution item for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryChartItem {
    pub category: String,
    pub file_count: i64,
    pub total_bytes: i64,
    pub percentage: f64,
}

/// Complete aggregated analytics response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageAnalyticsData {
    pub session_id: String,
    pub total_bytes: i64,
    pub total_files: i64,
    pub category_distribution: Vec<CategoryChartItem>,
    pub extension_distribution: Vec<ExtensionDistItem>,
    pub largest_directories: Vec<LargestDirItem>,
    pub application_storage: Vec<AppStorageItem>,
    pub developer_storage: Vec<DeveloperEcosystemSummary>,
    pub history_trends: Vec<StorageSnapshotRecord>,
}
