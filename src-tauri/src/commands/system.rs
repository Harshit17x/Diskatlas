use tauri::command;
use crate::error::Result;
use crate::models::system::SystemInfo;
use crate::services::system_inspector::SystemInspector;

#[command]
pub async fn get_system_info() -> Result<SystemInfo> {
    Ok(SystemInspector::inspect())
}

