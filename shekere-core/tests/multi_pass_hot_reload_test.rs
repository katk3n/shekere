use shekere_core::Config;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_hot_reload_multi_file_creation() {
    // Test that HotReloader can be created with multiple shader files
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create multiple test shader files
    let mut shader1 = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file 1");
    writeln!(shader1, "// First shader content").expect("Failed to write to temp file 1");

    let mut shader2 = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file 2");
    writeln!(shader2, "// Second shader content").expect("Failed to write to temp file 2");

    let shader_paths = vec![shader1.path().to_path_buf(), shader2.path().to_path_buf()];

    let result = shekere_core::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader for multiple files"
    );
}

#[test]
fn test_hot_reload_multi_file_invalid_path() {
    // Test that HotReloader handles invalid paths gracefully
    let shader_paths = vec![
        std::path::PathBuf::from("/non/existent/path1.wgsl"),
        std::path::PathBuf::from("/non/existent/path2.wgsl"),
    ];

    let result = shekere_core::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_err(),
        "Should fail to create HotReloader for invalid paths"
    );
}

#[test]
fn test_hot_reload_config_parsing_enabled() {
    // Test basic hot reload config parsing
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [[pipeline]]
        label = "Test Shader"
        file = "test.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse config");

    // Check that hot reload is enabled
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);

    // Check that pipeline entry exists
    assert_eq!(config.pipeline.len(), 1);
    assert_eq!(config.pipeline[0].file, "test.wgsl");
}

#[test]
fn test_hot_reload_config_parsing_disabled() {
    // Test that hot reload can be explicitly disabled
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = false
        
        [[pipeline]]
        label = "Test Shader"
        file = "test.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse config");

    // Check that hot reload is disabled
    assert!(config.hot_reload.is_some());
    assert!(!config.hot_reload.unwrap().enabled);
}

#[test]
fn test_hot_reload_config_parsing_missing() {
    // Test that missing hot reload config defaults to disabled
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [[pipeline]]
        label = "Test Shader"
        file = "test.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse config");

    // Check that hot reload is None (defaults to disabled)
    assert!(config.hot_reload.is_none());
}

#[test]
fn test_hot_reload_config_multi_pass() {
    // Test hot reload with multi-pass configuration
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [[pipeline]]
        label = "Scene"
        file = "scene.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
        
        [[pipeline]]
        label = "Blur"
        file = "blur.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse config");

    // Check that hot reload is enabled
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);

    // Check multi-pass configuration
    assert_eq!(config.pipeline.len(), 2);
    assert_eq!(config.pipeline[0].file, "scene.wgsl");
    assert_eq!(config.pipeline[1].file, "blur.wgsl");
}

#[test]
fn test_hot_reload_config_ping_pong() {
    // Test hot reload with ping-pong configuration
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [[pipeline]]
        label = "Game of Life"
        file = "life.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
        ping_pong = true
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse config");

    // Check that hot reload is enabled
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);

    // Check ping-pong configuration
    assert_eq!(config.pipeline.len(), 1);
    assert_eq!(config.pipeline[0].file, "life.wgsl");
    assert_eq!(config.pipeline[0].ping_pong, Some(true));
}

#[test]
fn test_hot_reload_config_persistent() {
    // Test hot reload with persistent texture configuration
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [[pipeline]]
        label = "Trail Effect"
        file = "trail.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
        persistent = true
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse config");

    // Check that hot reload is enabled
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);

    // Check persistent configuration
    assert_eq!(config.pipeline.len(), 1);
    assert_eq!(config.pipeline[0].file, "trail.wgsl");
    assert_eq!(config.pipeline[0].persistent, Some(true));
}

#[cfg(test)]
mod integration_tests {
    use shekere_core::hot_reload::MockHotReloader;

    #[test]
    fn test_mock_hot_reloader_for_testing() {
        // Test that MockHotReloader works for testing hot reload scenarios
        let mock_reloader = MockHotReloader::new();

        // Initially no changes
        assert!(!mock_reloader.check_for_changes());

        // Simulate file change
        mock_reloader.simulate_file_change();

        // Should detect change
        assert!(mock_reloader.check_for_changes());

        // Should reset after check
        assert!(!mock_reloader.check_for_changes());
    }

    #[test]
    fn test_mock_hot_reloader_multiple_changes() {
        // Test multiple rapid changes
        let mock_reloader = MockHotReloader::new();

        // Simulate multiple changes
        mock_reloader.simulate_file_change();
        mock_reloader.simulate_file_change();
        mock_reloader.simulate_file_change();

        // Should only trigger once
        assert!(mock_reloader.check_for_changes());
        assert!(!mock_reloader.check_for_changes());
    }
}
