//! Tauri IPC command handlers.

use lso_core::{ApprovalResponse, DiskUsageReport, Recommendation, RecommendationStatus};
use lso_sensor::{CpuInfo, MemoryInfo, ProcessInfo};
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

/// Collect all sensor data, store metrics in DB, and return recommendations.
#[tauri::command]
pub async fn scan_system(state: State<'_, AppState>) -> Result<Vec<Recommendation>, String> {
    let probes = lso_sensor::collect_all_probes();
    let mgr = state.recommendation_manager.lock().map_err(|e| e.to_string())?;

    for probe_result in &probes {
        mgr.store_metrics(&probe_result.metrics)
            .map_err(|e| e.to_string())?;
    }

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

/// Get current memory usage.
#[tauri::command]
pub async fn get_memory_usage() -> Result<MemoryInfo, String> {
    let probe = lso_sensor::MemoryProbe::for_host().map_err(|e| e.to_string())?;
    use lso_core::SystemProbe;
    let result = probe.collect().await.map_err(|e| e.to_string())?;

    let get_val = |suffix: &str| -> f64 {
        result
            .metrics
            .iter()
            .find(|m| m.name.ends_with(suffix))
            .map(|m| m.value)
            .unwrap_or(0.0)
    };

    Ok(MemoryInfo {
        total_bytes: get_val("::total_bytes") as u64,
        used_bytes: get_val("::used_bytes") as u64,
        available_bytes: get_val("::available_bytes") as u64,
        swap_total_bytes: get_val("::swap_total_bytes") as u64,
        swap_used_bytes: get_val("::swap_used_bytes") as u64,
        usage_percent: get_val("::usage_percent"),
        swap_percent: get_val("::swap_percent"),
    })
}

/// Get current CPU usage.
#[tauri::command]
pub async fn get_cpu_usage() -> Result<CpuInfo, String> {
    let probe = lso_sensor::CpuProbe::for_host().map_err(|e| e.to_string())?;
    use lso_core::SystemProbe;
    let result = probe.collect().await.map_err(|e| e.to_string())?;

    let get_val = |suffix: &str| -> f64 {
        result
            .metrics
            .iter()
            .find(|m| m.name.ends_with(suffix))
            .map(|m| m.value)
            .unwrap_or(0.0)
    };

    let core_count = get_val("::core_count") as u32;
    let per_core_percent: Vec<f64> = (0..core_count)
        .map(|i| {
            result
                .metrics
                .iter()
                .find(|m| m.name == format!("core_{i}::usage_percent"))
                .map(|m| m.value)
                .unwrap_or(0.0)
        })
        .collect();

    Ok(CpuInfo {
        core_count,
        per_core_percent,
        load_avg_1: get_val("::load_avg_1"),
        load_avg_5: get_val("::load_avg_5"),
        load_avg_15: get_val("::load_avg_15"),
    })
}

/// Get current process list.
#[tauri::command]
pub async fn get_process_list() -> Result<Vec<ProcessInfo>, String> {
    let probe = lso_sensor::ProcessListProbe::for_host().map_err(|e| e.to_string())?;
    use lso_core::SystemProbe;
    let result = probe.collect().await.map_err(|e| e.to_string())?;

    let mut processes: std::collections::HashMap<String, ProcessInfo> =
        std::collections::HashMap::new();

    for metric in &result.metrics {
        let parts: Vec<&str> = metric.name.splitn(2, "::").collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0];
        let field = parts[1];

        let entry = processes.entry(key.to_string()).or_insert_with(|| {
            let pid_name: Vec<&str> = key.splitn(2, ':').collect();
            let (pid, name) = if pid_name.len() == 2 {
                (pid_name[0].parse().unwrap_or(0), pid_name[1].to_string())
            } else {
                (0, key.to_string())
            };
            ProcessInfo {
                pid,
                name,
                cpu_percent: 0.0,
                memory_percent: 0.0,
                status: "unknown".to_string(),
            }
        });

        match field {
            "cpu_percent" => entry.cpu_percent = metric.value,
            "memory_percent" => entry.memory_percent = metric.value,
            _ => {}
        }
    }

    let mut list: Vec<ProcessInfo> = processes.into_values().collect();
    list.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
    Ok(list)
}
