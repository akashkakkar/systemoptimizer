//! LSO Tauri application library.

mod commands;
mod state;

use commands::{
    approve_recommendation, dismiss_recommendation, execute_cleanup, export_audit_log,
    get_app_version, get_audit_log, get_cpu_usage, get_disk_usage, get_memory_usage,
    get_process_list, get_recommendations, preflight_cleanup, reject_recommendation,
    rollback_action, scan_system,
};
use state::AppState;

pub fn run() {
    let app_state = AppState::init();

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_disk_usage,
            get_app_version,
            scan_system,
            get_recommendations,
            dismiss_recommendation,
            approve_recommendation,
            reject_recommendation,
            get_memory_usage,
            get_cpu_usage,
            get_process_list,
            preflight_cleanup,
            execute_cleanup,
            get_audit_log,
            export_audit_log,
            rollback_action,
        ])
        .run(tauri::generate_context!())
        .expect("error running LSO");
}
