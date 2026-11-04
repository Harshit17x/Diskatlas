use tauri::{command, State};

use crate::error::Result;
use crate::models::classification::{CategoryOverrideRule, StorageAnalysisProfile};
use crate::models::scan::ScanFilePage;
use crate::services::classifier::ClassifierService;
use crate::storage::DbPool;

#[command]
pub async fn get_storage_analysis(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<StorageAnalysisProfile> {
    ClassifierService::get_storage_analysis(&db, &session_id)
}

#[command]
pub async fn classify_session(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<StorageAnalysisProfile> {
    ClassifierService::classify_and_persist_session(&db, &session_id)
}

#[command]
pub async fn get_category_files(
    session_id: String,
    category: String,
    page: Option<u32>,
    page_size: Option<u32>,
    db: State<'_, DbPool>,
) -> Result<ScanFilePage> {
    let p = page.unwrap_or(1);
    let ps = page_size.unwrap_or(50);
    ClassifierService::get_category_files(&db, &session_id, &category, p, ps)
}

#[command]
pub async fn get_category_overrides(
    db: State<'_, DbPool>,
) -> Result<Vec<CategoryOverrideRule>> {
    ClassifierService::get_overrides(&db)
}

#[command]
pub async fn add_category_override(
    pattern: String,
    category: String,
    subcategory: Option<String>,
    db: State<'_, DbPool>,
) -> Result<CategoryOverrideRule> {
    ClassifierService::add_override(&db, &pattern, &category, subcategory.as_deref())
}

#[command]
pub async fn delete_category_override(
    id: i64,
    db: State<'_, DbPool>,
) -> Result<()> {
    ClassifierService::delete_override(&db, id)
}
