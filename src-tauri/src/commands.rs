//! Tauri IPC command handlers.

use lso_core::DiskUsageReport;

#[tauri::command]
pub async fn get_disk_usage() -> Result<Vec<DiskUsageReport>, String> {
    lso_sensor::get_disk_reports().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
