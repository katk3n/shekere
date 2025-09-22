// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod file_tree;

use commands::*;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_directory_tree,
            load_toml_config,
            start_preview,
            stop_preview,
            get_preview_status,
            get_frame_data,
            get_canvas_dimensions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
