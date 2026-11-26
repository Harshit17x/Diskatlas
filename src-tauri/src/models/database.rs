use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    pub status: String,
    pub migration_version: i64,
    pub db_path: String,
    pub tables_count: usize,
    pub message: Option<String>,
    pub db_size_bytes: Option<u64>,
    pub wal_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseIntegrityReport {
    pub ok: bool,
    pub details: Vec<String>,
    pub db_size_bytes: u64,
    pub wal_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseMaintenanceReport {
    pub ok: bool,
    pub previous_bytes: u64,
    pub current_bytes: u64,
    pub reclaimed_bytes: u64,
    pub message: String,
}
