use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The five distinct recommendation classification groups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationGroup {
    SafeToReview,
    ReviewRequired,
    SystemProtected,
    MoveOrArchive,
    UninstallOfficial,
}

impl RecommendationGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SafeToReview => "safe_to_review",
            Self::ReviewRequired => "review_required",
            Self::SystemProtected => "system_protected",
            Self::MoveOrArchive => "move_or_archive",
            Self::UninstallOfficial => "uninstall_official",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "safe_to_review" => Self::SafeToReview,
            "review_required" => Self::ReviewRequired,
            "system_protected" => Self::SystemProtected,
            "move_or_archive" => Self::MoveOrArchive,
            "uninstall_official" => Self::UninstallOfficial,
            _ => Self::ReviewRequired,
        }
    }
}

/// Risk level classification for cleanups and actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    None,
    Low,
    Moderate,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "low" => Self::Low,
            "moderate" => Self::Moderate,
            "high" => Self::High,
            "critical" => Self::Critical,
            _ => Self::Moderate,
        }
    }
}

/// A specific, evidence-backed storage recommendation item
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRecommendation {
    pub id: String,
    pub session_id: String,
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
    pub created_at: String,
}

/// A high-level finding explaining macro storage behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: String, // "info", "advisory", "warning"
    pub impact_bytes: i64,
    pub affected_items_count: usize,
    pub finding_type: String,
}

/// Largest storage consumer item with human-readable explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConsumer {
    pub path: String,
    pub name: String,
    pub size_bytes: i64,
    pub category: String,
    pub percentage_of_used: f64,
    pub is_directory: bool,
    pub explanation: String,
}

/// Aggregate summary item for a recommendation group
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupSummaryItem {
    pub group: String,
    pub count: usize,
    pub total_bytes: i64,
    pub recoverable_bytes: i64,
}

/// Complete Storage Intelligence profile containing findings, consumers, and recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageIntelligenceProfile {
    pub session_id: String,
    pub drive_id: String,
    pub total_scanned_bytes: i64,
    pub potential_reclaimable_bytes: i64,
    pub safe_reclaimable_bytes: i64,
    pub review_required_bytes: i64,
    pub system_protected_bytes: i64,
    pub findings: Vec<StorageFinding>,
    pub largest_consumers: Vec<StorageConsumer>,
    pub group_summaries: HashMap<String, GroupSummaryItem>,
}

/// Query parameters for paginating and filtering recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationsQuery {
    pub session_id: Option<String>,
    pub group: Option<String>,
    pub risk_level: Option<String>,
    pub search: Option<String>,
    pub page: u32,
    pub page_size: u32,
}

/// Paginated recommendation response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationsResponse {
    pub items: Vec<StorageRecommendation>,
    pub total_items: usize,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}
