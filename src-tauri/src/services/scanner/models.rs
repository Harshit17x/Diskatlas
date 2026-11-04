use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub session_id: String,
    pub drive_id: String,
    pub root_path: PathBuf,
    pub exclusions: Vec<String>,
    pub batch_size: usize,
}

#[derive(Debug, Clone)]
pub struct RawFileItem {
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

#[derive(Debug, Clone)]
pub struct RawDirItem {
    pub path: String,
    pub name: String,
    pub parent_path: Option<String>,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_size_bytes: u64,
    pub created_at: Option<i64>,
    pub modified_at: Option<i64>,
    pub is_hidden: bool,
    pub is_system: bool,
}

#[derive(Debug, Clone)]
pub struct RawErrorItem {
    pub path: String,
    pub error_type: String,
    pub message: String,
}

