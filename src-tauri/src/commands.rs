//! Tauri IPC command handlers.

use lso_core::{ApprovalResponse, DiskUsageReport, Recommendation, RecommendationStatus};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub async fn get_disk_usage() -> Result<Vec<DiskUsageReport>, String> {
    lso_sensor::get_disk_reports().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Scan the system and generate new recommendations.
#[tauri::command]
pub async fn scan_system(state: State<'_, AppState>) -> Result<Vec<Recommendation>, String> {
    let probes = lso_sensor::collect_all_probes();
    let mgr = state.recommendation_manager.lock().map_err(|e| e.to_string())?;
    mgr.scan(&probes).map_err(|e| e.to_string())
}

/// Get all recommendations, optionally filtered by status.
#[tauri::command]
pub async fn get_recommendations(
    state: State<'_, AppState>,
    status: Option<RecommendationStatus>,
) -> Result<Vec<Recommendation>, String> {
    let mgr = state.recommendation_manager.lock().map_err(|e| e.to_string())?;
    mgr.list(status).map_err(|e| e.to_string())
}

/// Dismiss (reject) a recommendation.
#[tauri::command]
pub async fn dismiss_recommendation(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let mgr = state.recommendation_manager.lock().map_err(|e| e.to_string())?;
    mgr.dismiss(&id).map_err(|e| e.to_string())
}

/// Approve a recommendation.
#[tauri::command]
pub async fn approve_recommendation(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApprovalResponse, String> {
    let mgr = state.recommendation_manager.lock().map_err(|e| e.to_string())?;
    mgr.approve(&id).map_err(|e| e.to_string())
}

/// Reject a recommendation with optional reason.
#[tauri::command]
pub async fn reject_recommendation(
    state: State<'_, AppState>,
    id: String,
    reason: Option<String>,
) -> Result<(), String> {
    let mgr = state.recommendation_manager.lock().map_err(|e| e.to_string())?;
    mgr.reject(&id, reason).map_err(|e| e.to_string())
}
