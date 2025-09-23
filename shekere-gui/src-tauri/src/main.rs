// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod file_tree;

use commands::*;

fn main() {
    // Initialize logging with filtered output - exclude verbose wgpu logs
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shekere_gui", log::LevelFilter::Debug)
        .filter_module("shekere_core", log::LevelFilter::Info) // Include shekere_core info logs for shader loading
        .filter_module("shekere_core::pipeline", log::LevelFilter::Debug) // Detailed pipeline logs
        .filter_module("wgpu_core", log::LevelFilter::Warn)
        .filter_module("wgpu_hal", log::LevelFilter::Warn)
        .init();

    log::info!("Starting shekere-gui application");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_directory_tree,
            load_toml_config,
            start_preview,
            stop_preview,
            get_preview_status,
            get_frame_data,
            get_frame_data_with_dimensions,
            get_canvas_dimensions,
            handle_mouse_input
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
