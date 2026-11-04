use serde::{Deserialize, Serialize};
use crate::models::intelligence::{RecommendationGroup, RiskLevel};

/// A single candidate item in a cleanup plan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItemCandidate {
    pub id: String,
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub category: String,
    pub group: RecommendationGroup,
    pub risk_level: RiskLevel,
    pub reason: String,
    pub confidence: f64,
    pub recoverable_bytes: i64,
    pub expected_consequences: String,
    pub recommended_action: String,
    pub is_cloud_synced: bool,
    pub cloud_warning: Option<String>,
    pub is_blocked: bool,
    pub block_reason: Option<String>,
    pub selected: bool,
    pub exists_on_disk: bool,
}

/// Aggregated cleanup plan presented to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlan {
    pub batch_id: String,
    pub session_id: String,
    pub candidates: Vec<CleanupItemCandidate>,
    pub total_items: usize,
    pub total_bytes: i64,
    pub selected_items: usize,
    pub selected_bytes: i64,
    pub safe_items: usize,
    pub safe_bytes: i64,
    pub review_required_items: usize,
    pub review_required_bytes: i64,
    pub blocked_items: usize,
    pub has_cloud_synced: bool,
}

/// Request to execute cleanup on user-selected items
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCleanupRequest {
    pub batch_id: String,
    pub session_id: String,
    pub item_ids: Vec<String>,
    pub use_recycle_bin: bool,
    pub confirmation_phrase: String, // Must be "CONFIRM_CLEANUP"
}

/// Individual item cleanup result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOperationResult {
    pub id: String,
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub status: String, // "success", "failed", "blocked_security", "in_use", "permission_denied", "not_found"
    pub cleanup_method: String, // "recycle_bin" or "permanent_delete"
    pub is_reversible: bool,
    pub error_message: Option<String>,
}

/// Response returned after executing cleanup
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCleanupResponse {
    pub batch_id: String,
    pub total_attempted: usize,
    pub successful_count: usize,
    pub failed_count: usize,
    pub bytes_reclaimed: i64,
    pub results: Vec<CleanupOperationResult>,
}

/// Audit log record persisted in SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupAuditRecord {
    pub id: String,
    pub batch_id: String,
    pub session_id: Option<String>,
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub category: String,
    pub risk_level: String,
    pub cleanup_method: String,
    pub status: String,
    pub is_reversible: bool,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// Query for cleanup audit history
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupAuditQuery {
    pub session_id: Option<String>,
    pub status: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

/// Paginated audit history response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupAuditResponse {
    pub items: Vec<CleanupAuditRecord>,
    pub total_items: usize,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}
