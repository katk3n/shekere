use std::io::Write;
use tempfile::{NamedTempFile, TempDir};
use shekere_core::{Config, hot_reload::{HotReloader, MockHotReloader}};

#[test]
fn test_mock_hot_reloader_error_scenarios() {
    // Test that MockHotReloader handles error scenarios correctly
    let mock_reloader = MockHotReloader::new();

    // Test initial state
    assert!(!mock_reloader.check_for_changes());

    // Simulate multiple rapid changes (should still only trigger once)
    mock_reloader.simulate_file_change();
    mock_reloader.simulate_file_change();
    mock_reloader.simulate_file_change();

    // Should only trigger once despite multiple changes
    assert!(mock_reloader.check_for_changes());
    assert!(!mock_reloader.check_for_changes());
}

#[test]
fn test_hot_reload_with_valid_shader_syntax() {
    // Test creating HotReloader with syntactically valid shaders
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut valid_shader = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        valid_shader,
        r#"
        @vertex
        fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {{
            let x = f32(vertex_index) - 1.0;
            let y = f32(vertex_index & 1u) * 2.0 - 1.0;
            return vec4<f32>(x, y, 0.0, 1.0);
        }}
        
        @fragment
        fn fs_main() -> @location(0) vec4<f32> {{
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }}
        "#
    )
    .expect("Failed to write valid shader");

    let shader_paths = vec![valid_shader.path().to_path_buf()];
    let result = HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader with valid shader"
    );
}

#[test]
fn test_hot_reload_with_invalid_shader_files() {
    // Test creating HotReloader with files that contain invalid WGSL syntax
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut invalid_shader = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        invalid_shader,
        r#"
        // This is intentionally invalid WGSL
        invalid syntax here!
        @fragment
        fn fs_main() -> @location(0) vec4<f32> {{
            missing_function_call();
            return undefined_variable;
        }}
        "#
    )
    .expect("Failed to write invalid shader");

    let shader_paths = vec![invalid_shader.path().to_path_buf()];

    // File watching should still work even if the content is invalid
    // The actual compilation error will be caught during pipeline creation
    let result = HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "HotReloader creation should succeed even with invalid shader content"
    );
}

#[test]
fn test_hot_reload_config_for_error_handling() {
    // Test configuration that would be used in error handling scenarios
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [[pipeline]]
        label = "Error Test Shader"
        file = "error_test.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse config");

    // Verify hot reload is properly configured
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);

    // Verify pipeline configuration
    assert_eq!(config.pipeline.len(), 1);
    assert_eq!(config.pipeline[0].file, "error_test.wgsl");
    assert_eq!(config.pipeline[0].label, "Error Test Shader");
}

#[test]
fn test_hot_reload_with_multi_pass_error_scenario() {
    // Test configuration for multi-pass hot reload error handling
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [[pipeline]]
        label = "First Pass"
        file = "pass1.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
        
        [[pipeline]]
        label = "Second Pass"
        file = "pass2.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config =
        toml::from_str(config_content).expect("Failed to parse multi-pass config");

    // Verify multi-pass configuration
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);
    assert_eq!(config.pipeline.len(), 2);

    // This configuration should be handled correctly even if one shader has errors
    assert_eq!(config.pipeline[0].file, "pass1.wgsl");
    assert_eq!(config.pipeline[1].file, "pass2.wgsl");
}

#[test]
fn test_error_recovery_simulation() {
    // Simulate the error recovery workflow using MockHotReloader
    let mock_reloader = MockHotReloader::new();

    // Phase 1: Normal operation (no changes)
    assert!(!mock_reloader.check_for_changes());

    // Phase 2: File modified (error introduced)
    mock_reloader.simulate_file_change();
    assert!(mock_reloader.check_for_changes());
    // At this point, the system would attempt compilation, fail, and keep old pipeline

    // Phase 3: No more changes detected after handling
    assert!(!mock_reloader.check_for_changes());

    // Phase 4: File fixed (valid shader)
    mock_reloader.simulate_file_change();
    assert!(mock_reloader.check_for_changes());
    // At this point, the system would successfully compile and update pipeline

    // Phase 5: Back to normal operation
    assert!(!mock_reloader.check_for_changes());
}
