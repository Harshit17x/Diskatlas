use tauri::{command, State};

use crate::error::Result;
use crate::models::classification::{CategoryOverrideRule, StorageAnalysisProfile};
use crate::models::scan::ScanFilePage;
use crate::services::classifier::ClassifierService;
use crate::storage::DbPool;

#[command]
pub async fn get_storage_analysis(
    session_id: Option<String>,
    db: State<'_, DbPool>,
) -> Result<StorageAnalysisProfile> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        ClassifierService::get_storage_analysis(&pool, session_id.as_deref())
    })
    .await
    .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[command]
pub async fn classify_session(
    session_id: String,
    db: State<'_, DbPool>,
) -> Result<StorageAnalysisProfile> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        ClassifierService::classify_and_persist_session(&pool, &session_id)
    })
    .await
    .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
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
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        ClassifierService::get_category_files(&pool, &session_id, &category, p, ps)
    })
    .await
    .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[command]
pub async fn get_category_overrides(
    db: State<'_, DbPool>,
) -> Result<Vec<CategoryOverrideRule>> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || ClassifierService::get_overrides(&pool))
        .await
        .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[command]
pub async fn add_category_override(
    pattern: String,
    category: String,
    subcategory: Option<String>,
    db: State<'_, DbPool>,
) -> Result<CategoryOverrideRule> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || {
        ClassifierService::add_override(&pool, &pattern, &category, subcategory.as_deref())
    })
    .await
    .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}

#[command]
pub async fn delete_category_override(
    id: i64,
    db: State<'_, DbPool>,
) -> Result<()> {
    let pool = (*db).clone();
    tokio::task::spawn_blocking(move || ClassifierService::delete_override(&pool, id))
        .await
        .map_err(|e| crate::error::AppError::System(format!("Task join error: {}", e)))?
}
