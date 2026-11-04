use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseHealth {
    pub status: String,
    pub migration_version: i64,
    pub db_path: String,
    pub tables_count: usize,
    pub message: Option<String>,
}

