use crate::error::{AppError, Result};
use crate::models::cleanup::ApplicationsInventoryReport;
use crate::services::applications::ApplicationsService;

#[tauri::command]
pub async fn inspect_installed_applications() -> Result<ApplicationsInventoryReport> {
    tokio::task::spawn_blocking(ApplicationsService::inspect_installed_applications)
        .await
        .map_err(|e| AppError::System(format!("Application inspection task failed: {}", e)))?
}
