use serde::{Deserialize, Serialize};
use crate::models::scan::{ScanFileItem, ScanStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSessionRecord {
    pub id: String,
    pub drive_id: String,
    pub drive_label: Option<String>,
    pub root_path: String,
    pub status: ScanStatus,
    pub total_files: u64,
    pub total_directories: u64,
    pub total_bytes: u64,
    pub error_count: u64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub duration_millis: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSessionPage {
    pub items: Vec<ScanSessionRecord>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategorySummaryItem {
    pub category: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub percentage_bytes: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryRecord {
    pub path: String,
    pub name: String,
    pub total_size_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanErrorItem {
    pub path: String,
    pub error_type: String,
    pub message: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSnapshotRecord {
    pub id: String,
    pub session_id: Option<String>,
    pub drive_id: String,
    pub label: String,
    pub root_path: String,
    pub total_files: u64,
    pub total_directories: u64,
    pub total_bytes: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetails {
    pub session: ScanSessionRecord,
    pub categories: Vec<CategorySummaryItem>,
    pub top_directories: Vec<DirectoryRecord>,
    pub error_samples: Vec<ScanErrorItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDeltaItem {
    pub category: String,
    pub baseline_bytes: u64,
    pub target_bytes: u64,
    pub delta_bytes: i64,
    pub baseline_files: u64,
    pub target_files: u64,
    pub delta_files: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryDeltaItem {
    pub path: String,
    pub baseline_size_bytes: u64,
    pub target_size_bytes: u64,
    pub delta_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileModifiedItem {
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub baseline_size_bytes: u64,
    pub target_size_bytes: u64,
    pub size_delta_bytes: i64,
    pub modified_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanComparisonResult {
    pub baseline_session: ScanSessionRecord,
    pub target_session: ScanSessionRecord,
    pub size_delta_bytes: i64,
    pub files_delta: i64,
    pub directories_delta: i64,
    pub duration_delta_millis: Option<i64>,
    pub category_deltas: Vec<CategoryDeltaItem>,
    pub directory_deltas: Vec<DirectoryDeltaItem>,
    pub added_files_count: u64,
    pub removed_files_count: u64,
    pub modified_files_count: u64,
    pub added_files_sample: Vec<ScanFileItem>,
    pub removed_files_sample: Vec<ScanFileItem>,
    pub modified_files_sample: Vec<FileModifiedItem>,
}
