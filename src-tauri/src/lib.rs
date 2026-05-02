//! LSO Tauri application library.

mod commands;

use commands::{get_app_version, get_disk_usage};

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_disk_usage, get_app_version])
        .run(tauri::generate_context!())
        .expect("error running LSO");
}
