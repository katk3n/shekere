// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod file_tree;
mod native_renderer;
mod window_manager;

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

    log::info!("Starting shekere-gui application with Tauri on main thread");

    // Initialize the window manager communication system
    window_manager::init_global_communication();

    // Run Tauri on the main thread (this owns the main thread)
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_directory_tree,
            load_toml_config,
            load_shader_content,
            start_preview,        // Compatibility alias for start_native_preview
            start_native_preview, // Headless rendering
            stop_preview,
            get_preview_status,
            handle_mouse_input,
            check_webgpu_availability, // WebGPU diagnostics
            // New window management commands
            start_preview_window,
            stop_preview_window,
            get_preview_window_status,
            handle_preview_window_mouse,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
