use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_hot_reload_configuration_parsing() {
    // Test that hot reload configuration is properly parsed from TOML
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

    let config: shekere::Config = toml::from_str(config_content).expect("Failed to parse config");

    // Verify hot reload is properly configured
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);
}

#[test]
fn test_hot_reload_disabled_by_default() {
    // Test that hot reload is disabled when not specified in TOML
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

    let config: shekere::Config = toml::from_str(config_content).expect("Failed to parse config");

    // Hot reload should be None when not specified
    assert!(config.hot_reload.is_none());
}

#[test]
fn test_hot_reloader_creation_single_file() {
    // Test creating HotReloader for a single shader file
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let mut shader_file = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(
        shader_file,
        r#"
        fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
            return vec4<f32>(1.0, 0.0, 0.0, 1.0);
        }}
        "#
    )
    .expect("Failed to write shader");

    let result = shekere::hot_reload::HotReloader::new(shader_file.path());
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader for single file"
    );

    let reloader = result.unwrap();
    // Initially no changes
    assert!(!reloader.check_for_changes());
}

#[test]
fn test_hot_reloader_creation_multi_file() {
    // Test creating HotReloader for multiple shader files (multi-pass)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    let mut shader_file1 = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(shader_file1, "// Shader 1").expect("Failed to write shader 1");

    let mut shader_file2 = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file");
    writeln!(shader_file2, "// Shader 2").expect("Failed to write shader 2");

    let shader_paths = vec![
        shader_file1.path().to_path_buf(),
        shader_file2.path().to_path_buf(),
    ];

    let result = shekere::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader for multiple files"
    );

    let reloader = result.unwrap();
    // Initially no changes
    assert!(!reloader.check_for_changes());
}

#[test]
fn test_multi_pass_hot_reload_config() {
    // Test configuration for multi-pass hot reload
    let config_content = r#"
        [window]
        width = 800
        height = 600

        [hot_reload]
        enabled = true

        [[pipeline]]
        label = "Pass 0"
        file = "scene.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"

        [[pipeline]]
        label = "Pass 1"
        file = "blur.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: shekere::Config =
        toml::from_str(config_content).expect("Failed to parse multi-pass config");

    // Verify configuration
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);
    assert_eq!(config.pipeline.len(), 2);
    assert_eq!(config.pipeline[0].file, "scene.wgsl");
    assert_eq!(config.pipeline[1].file, "blur.wgsl");
}

#[test]
fn test_persistent_hot_reload_config() {
    // Test configuration for persistent rendering with hot reload
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

    let config: shekere::Config =
        toml::from_str(config_content).expect("Failed to parse persistent config");

    // Verify configuration
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);
    assert_eq!(config.pipeline.len(), 1);
    assert_eq!(config.pipeline[0].file, "trail.wgsl");
    assert_eq!(config.pipeline[0].persistent, Some(true));
}
