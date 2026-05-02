//! LSO application entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    lso_app_lib::run();
}
