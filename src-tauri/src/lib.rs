//! LSO Tauri application library.

mod commands;
mod state;

use commands::{
    approve_recommendation, dismiss_recommendation, get_app_version, get_cpu_usage,
    get_disk_usage, get_memory_usage, get_process_list, get_recommendations,
    reject_recommendation, scan_system,
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
        ])
        .run(tauri::generate_context!())
        .expect("error running LSO");
}
