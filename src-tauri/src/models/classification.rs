use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageCategory {
    OperatingSystem,
    Applications,
    Developer,
    Temporary,
    Documents,
    Images,
    Videos,
    Audio,
    Archives,
    Browsers,
    Cloud,
    Gaming,
    Backups,
    VirtualMachines,
    Hidden,
    Unknown,
}

impl StorageCategory {
    pub fn id(&self) -> &'static str {
        match self {
            StorageCategory::OperatingSystem => "os",
            StorageCategory::Applications => "applications",
            StorageCategory::Developer => "developer",
            StorageCategory::Temporary => "temporary",
            StorageCategory::Documents => "documents",
            StorageCategory::Images => "images",
            StorageCategory::Videos => "videos",
            StorageCategory::Audio => "audio",
            StorageCategory::Archives => "archives",
            StorageCategory::Browsers => "browsers",
            StorageCategory::Cloud => "cloud",
            StorageCategory::Gaming => "gaming",
            StorageCategory::Backups => "backups",
            StorageCategory::VirtualMachines => "vm_emulators",
            StorageCategory::Hidden => "hidden",
            StorageCategory::Unknown => "unknown",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            StorageCategory::OperatingSystem => "Operating System",
            StorageCategory::Applications => "Applications",
            StorageCategory::Developer => "Developer Storage",
            StorageCategory::Temporary => "Temporary Files",
            StorageCategory::Documents => "Personal Documents",
            StorageCategory::Images => "Images",
            StorageCategory::Videos => "Videos",
            StorageCategory::Audio => "Audio",
            StorageCategory::Archives => "Downloads & Archives",
            StorageCategory::Browsers => "Browser Storage",
            StorageCategory::Cloud => "Cloud Synchronization",
            StorageCategory::Gaming => "Gaming",
            StorageCategory::Backups => "Backups & Recovery",
            StorageCategory::VirtualMachines => "Virtual Machines & Emulators",
            StorageCategory::Hidden => "Hidden Storage",
            StorageCategory::Unknown => "Unknown Storage",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "os" => StorageCategory::OperatingSystem,
            "applications" => StorageCategory::Applications,
            "developer" => StorageCategory::Developer,
            "temporary" => StorageCategory::Temporary,
            "documents" => StorageCategory::Documents,
            "images" => StorageCategory::Images,
            "videos" => StorageCategory::Videos,
            "audio" => StorageCategory::Audio,
            "archives" => StorageCategory::Archives,
            "browsers" => StorageCategory::Browsers,
            "cloud" => StorageCategory::Cloud,
            "gaming" => StorageCategory::Gaming,
            "backups" => StorageCategory::Backups,
            "vm_emulators" => StorageCategory::VirtualMachines,
            "hidden" => StorageCategory::Hidden,
            _ => StorageCategory::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationResult {
    pub category: String,
    pub subcategory: Option<String>,
    pub app_name: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubcategoryStatItem {
    pub subcategory: String,
    pub file_count: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatItem {
    pub app_name: String,
    pub file_count: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryStatItem {
    pub category: String,
    pub label: String,
    pub file_count: u64,
    pub dir_count: u64,
    pub total_bytes: u64,
    pub percentage_bytes: f64,
    pub avg_confidence: f64,
    pub subcategories: Vec<SubcategoryStatItem>,
    pub top_apps: Vec<AppStatItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageAnalysisProfile {
    pub session_id: String,
    pub drive_id: String,
    pub total_files: u64,
    pub total_bytes: u64,
    pub categories: Vec<CategoryStatItem>,
    pub classified_percent: f64,
    pub overall_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryOverrideRule {
    pub id: i64,
    pub pattern: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub created_at: String,
}
