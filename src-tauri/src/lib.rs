//! LSO Tauri application library.

mod commands;
mod state;

use commands::{
    approve_recommendation, dismiss_recommendation, get_app_version, get_disk_usage,
    get_recommendations, reject_recommendation, scan_system,
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
        ])
        .run(tauri::generate_context!())
        .expect("error running LSO");
}
