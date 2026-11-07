use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanStatus {
    Queued,
    Running,
    Paused,
    Completed,
    CompletedWithErrors,
    Cancelled,
    Failed,
    Interrupted,
}

impl ScanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanStatus::Queued => "queued",
            ScanStatus::Running => "running",
            ScanStatus::Paused => "paused",
            ScanStatus::Completed => "completed",
            ScanStatus::CompletedWithErrors => "completed_with_errors",
            ScanStatus::Cancelled => "cancelled",
            ScanStatus::Failed => "failed",
            ScanStatus::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub session_id: String,
    pub drive_id: String,
    pub root_path: String,
    pub status: ScanStatus,
    pub files_scanned: u64,
    pub dirs_scanned: u64,
    pub bytes_scanned: u64,
    pub error_count: u64,
    pub current_path: String,
    #[serde(default)]
    pub current_file: Option<String>,
    #[serde(default)]
    pub recent_files: Vec<String>,
    pub elapsed_millis: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFileItem {
    pub id: i64,
    pub session_id: String,
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub parent_path: String,
    pub size_bytes: u64,
    pub allocated_bytes: Option<u64>,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub is_hidden: bool,
    pub is_system: bool,
    pub is_symlink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFilePage {
    pub items: Vec<ScanFileItem>,
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub session_id: String,
    pub drive_id: String,
    pub root_path: String,
    pub status: ScanStatus,
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_bytes: u64,
    pub error_count: u64,
    pub duration_millis: u64,
}

