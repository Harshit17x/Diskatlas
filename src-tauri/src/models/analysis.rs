use serde::{Deserialize, Serialize};

/// Represents a single large file item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileItem {
    pub id: i64,
    pub session_id: String,
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size_bytes: i64,
    pub category: Option<String>,
    pub modified_at: Option<i64>,
    pub is_hidden: bool,
    pub is_system: bool,
}

/// Query parameters for filtering and paginating large files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFilesQuery {
    pub session_id: Option<String>,
    pub min_size_bytes: Option<i64>,
    pub extension: Option<String>,
    pub path_filter: Option<String>,
    pub sort_by: Option<String>, // "size_desc", "size_asc", "modified_desc", "name_asc"
    pub page: u32,
    pub page_size: u32,
}

/// Paginated large files result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LargeFilesResponse {
    pub items: Vec<LargeFileItem>,
    pub total_items: i64,
    pub total_bytes: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Represents an audited hidden or system-protected file or directory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenItem {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub is_directory: bool,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
    pub hidden_type: String,       // "system_protected", "user_hidden", "dotfile", "app_hidden"
    pub protection_level: String,  // "system_protected", "user_managed", "app_managed"
    pub safety_note: String,
}

/// Query for hidden files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenFilesQuery {
    pub session_id: Option<String>,
    pub filter_type: Option<String>, // "all", "system_protected", "user_hidden", "dotfile", "app_hidden"
    pub search: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

/// Paginated hidden files result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenFilesResponse {
    pub items: Vec<HiddenItem>,
    pub total_items: i64,
    pub total_bytes: i64,
    pub system_protected_bytes: i64,
    pub user_hidden_bytes: i64,
    pub dotfiles_bytes: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// Represents an individual file in a duplicate cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFileItem {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
    pub created_at: Option<i64>,
}

/// A cluster of confirmed identical files sharing the exact same cryptographic hash
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub hash: String,
    pub unit_size_bytes: i64,
    pub copies_count: usize,
    pub wasted_bytes: i64,
    pub sample_name: String,
    pub files: Vec<DuplicateFileItem>,
}

/// Parameters for duplicate search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateSearchQuery {
    pub session_id: Option<String>,
    pub min_size_bytes: Option<i64>,
    pub extension: Option<String>,
    pub limit_groups: Option<u32>,
}

/// Overall duplicate search summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateSearchResponse {
    pub groups: Vec<DuplicateGroup>,
    pub total_duplicate_files: usize,
    pub total_wasted_bytes: i64,
    pub groups_count: usize,
    pub hashing_algorithm: String,
}

/// Heuristically flagged forgotten file
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgottenFileItem {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub size_bytes: i64,
    pub modified_at: Option<i64>,
    pub age_days: i64,
    pub signal_type: String,       // "old_download", "old_installer", "old_backup", "old_archive", "stale_build_artifact"
    pub safety_heuristic: String,
}

/// Query for forgotten files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgottenFilesQuery {
    pub session_id: Option<String>,
    pub min_age_days: Option<i64>,
    pub signal_type: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

/// Response for forgotten files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgottenFilesResponse {
    pub items: Vec<ForgottenFileItem>,
    pub total_items: i64,
    pub total_bytes: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// An unclassified file or folder
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownStorageItem {
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: i64,
    pub file_count: i64,
    pub extension: Option<String>,
    pub modified_at: Option<i64>,
    pub confidence: f64,
}

/// Aggregate response for unknown storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnknownStorageResponse {
    pub items: Vec<UnknownStorageItem>,
    pub total_unknown_bytes: i64,
    pub total_unknown_files: i64,
    pub top_unknown_directories: Vec<UnknownStorageItem>,
}

