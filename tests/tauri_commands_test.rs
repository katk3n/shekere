// tests/tauri_commands_test.rs
// TDD Tests for Tauri Commands - Phase 3: GUI Foundation
// These tests define the expected Tauri command API for the GUI

use std::path::Path;

#[cfg(test)]
mod tauri_command_structure_tests {
    use super::*;

    #[test]
    fn test_tauri_commands_file_exists() {
        // Test that commands.rs file exists in the Tauri backend
        let commands_path = Path::new("shekere-gui/src-tauri/src/commands.rs");
        assert!(commands_path.exists(), "shekere-gui/src-tauri/src/commands.rs should exist");
    }

    #[test]
    fn test_file_tree_module_exists() {
        // Test that file_tree.rs module exists
        let file_tree_path = Path::new("shekere-gui/src-tauri/src/file_tree.rs");
        assert!(file_tree_path.exists(), "shekere-gui/src-tauri/src/file_tree.rs should exist");
    }
}

#[cfg(test)]
mod tauri_command_api_tests {
    use std::fs;

    #[test]
    fn test_get_directory_tree_command_exists() {
        // Test that get_directory_tree command is defined
        let commands_content = fs::read_to_string("shekere-gui/src-tauri/src/commands.rs")
            .expect("Should be able to read commands.rs");

        assert!(commands_content.contains("get_directory_tree"),
               "commands.rs should contain get_directory_tree function");

        assert!(commands_content.contains("#[tauri::command]"),
               "commands.rs should have tauri::command attributes");
    }

    #[test]
    fn test_load_toml_config_command_exists() {
        // Test that load_toml_config command is defined
        let commands_content = fs::read_to_string("shekere-gui/src-tauri/src/commands.rs")
            .expect("Should be able to read commands.rs");

        assert!(commands_content.contains("load_toml_config"),
               "commands.rs should contain load_toml_config function");
    }

    #[test]
    fn test_preview_commands_exist() {
        // Test that start_preview and stop_preview commands are defined
        let commands_content = fs::read_to_string("shekere-gui/src-tauri/src/commands.rs")
            .expect("Should be able to read commands.rs");

        assert!(commands_content.contains("start_preview"),
               "commands.rs should contain start_preview function");

        assert!(commands_content.contains("stop_preview"),
               "commands.rs should contain stop_preview function");
    }

    #[test]
    fn test_commands_use_proper_types() {
        // Test that commands use appropriate types and error handling
        let commands_content = fs::read_to_string("shekere-gui/src-tauri/src/commands.rs")
            .expect("Should be able to read commands.rs");

        assert!(commands_content.contains("Result<"),
               "commands.rs should use Result types for error handling");
    }
}

#[cfg(test)]
mod tauri_main_integration_tests {
    use std::fs;

    #[test]
    fn test_main_rs_imports_commands() {
        // Test that main.rs imports and registers the commands
        let main_content = fs::read_to_string("shekere-gui/src-tauri/src/main.rs")
            .expect("Should be able to read main.rs");

        assert!(main_content.contains("commands"),
               "main.rs should import commands module");
    }

    #[test]
    fn test_main_rs_uses_shekere_core() {
        // Test that main.rs uses shekere-core
        let main_content = fs::read_to_string("shekere-gui/src-tauri/src/main.rs")
            .expect("Should be able to read main.rs");

        assert!(main_content.contains("shekere_core") || main_content.contains("shekere-core"),
               "main.rs should import shekere_core");
    }

    #[test]
    fn test_tauri_app_builder_has_commands() {
        // Test that Tauri app builder includes our commands
        let main_content = fs::read_to_string("shekere-gui/src-tauri/src/main.rs")
            .expect("Should be able to read main.rs");

        assert!(main_content.contains("invoke_handler"),
               "main.rs should use invoke_handler for commands");

        assert!(main_content.contains("get_directory_tree") ||
                main_content.contains("commands::"),
               "main.rs should register get_directory_tree command");
    }
}

#[cfg(test)]
mod file_tree_implementation_tests {
    use std::fs;

    #[test]
    fn test_file_tree_has_directory_traversal() {
        // Test that file_tree.rs has directory traversal functionality
        let file_tree_content = fs::read_to_string("shekere-gui/src-tauri/src/file_tree.rs")
            .expect("Should be able to read file_tree.rs");

        assert!(file_tree_content.contains("std::fs") ||
                file_tree_content.contains("read_dir"),
               "file_tree.rs should use filesystem operations");
    }

    #[test]
    fn test_file_tree_has_file_type_detection() {
        // Test that file_tree.rs can detect file types
        let file_tree_content = fs::read_to_string("shekere-gui/src-tauri/src/file_tree.rs")
            .expect("Should be able to read file_tree.rs");

        assert!(file_tree_content.contains("extension") ||
                file_tree_content.contains(".toml") ||
                file_tree_content.contains(".wgsl"),
               "file_tree.rs should handle file extensions");
    }

    #[test]
    fn test_file_tree_returns_structured_data() {
        // Test that file_tree.rs returns structured data (likely JSON-serializable)
        let file_tree_content = fs::read_to_string("shekere-gui/src-tauri/src/file_tree.rs")
            .expect("Should be able to read file_tree.rs");

        assert!(file_tree_content.contains("serde") ||
                file_tree_content.contains("Serialize") ||
                file_tree_content.contains("Deserialize"),
               "file_tree.rs should use serde for serialization");
    }
}