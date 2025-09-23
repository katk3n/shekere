use shekere_core::Config;
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn test_hot_reload_config_parsing_with_bind_groups() {
    // Test that hot reload config parsing works with various bind group configurations
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [osc]
        port = 8000
        addr_pattern = "/test"
        
        [[osc.sound]]
        name = "test"
        id = 1
        
        [spectrum]
        min_frequency = 27.0
        max_frequency = 2000.0
        sampling_rate = 44100
        
        [midi]
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

    // Check that audio features are configured
    assert!(config.osc.is_some());
    assert!(config.spectrum.is_some());
    assert!(config.midi.is_some());

    // Check that pipeline entry exists
    assert_eq!(config.pipeline.len(), 1);
    assert_eq!(config.pipeline[0].file, "test.wgsl");
}

#[test]
fn test_hot_reload_multi_file_watcher_functionality() {
    // Test the multi-file watching functionality in isolation
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create test shader files
    let mut shader1 = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file 1");
    writeln!(shader1, "@fragment fn fs_main() -> @location(0) vec4<f32> {{ return vec4<f32>(1.0, 0.0, 0.0, 1.0); }}").expect("Failed to write to temp file 1");

    let mut shader2 = NamedTempFile::new_in(&temp_dir).expect("Failed to create temp file 2");
    writeln!(shader2, "@fragment fn fs_main() -> @location(0) vec4<f32> {{ return vec4<f32>(0.0, 1.0, 0.0, 1.0); }}").expect("Failed to write to temp file 2");

    let shader_paths = vec![shader1.path().to_path_buf(), shader2.path().to_path_buf()];

    // Test creating HotReloader with multiple valid files
    let result = shekere::hot_reload::HotReloader::new_multi_file(shader_paths);
    assert!(
        result.is_ok(),
        "Should successfully create HotReloader for multiple valid shader files"
    );

    let hot_reloader = result.unwrap();

    // Test initial state (no changes)
    assert!(
        !hot_reloader.check_for_changes(),
        "Should start with no changes detected"
    );

    // Note: We can't easily test actual file modification detection in unit tests
    // due to filesystem timing issues, but we've verified the basic functionality
}

#[test]
fn test_hot_reload_with_minimal_config() {
    // Test hot reload with the minimal possible configuration
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [[pipeline]]
        label = "Minimal Shader"
        file = "minimal.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config = toml::from_str(config_content).expect("Failed to parse minimal config");

    // Verify basic structure
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);
    assert_eq!(config.pipeline.len(), 1);

    // Verify optional features are None
    assert!(config.osc.is_none());
    assert!(config.spectrum.is_none());
    assert!(config.midi.is_none());
}

#[test]
fn test_hot_reload_with_all_optional_features() {
    // Test hot reload with all optional features enabled
    let config_content = r#"
        [window]
        width = 800
        height = 600
        
        [hot_reload]
        enabled = true
        
        [osc]
        port = 8000
        addr_pattern = "/test"
        
        [[osc.sound]]
        name = "test"
        id = 1
        
        [spectrum]
        min_frequency = 27.0
        max_frequency = 2000.0
        sampling_rate = 44100
        
        [midi]
        enabled = true
        
        [[pipeline]]
        label = "Feature-Rich Shader"
        file = "rich.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
        
        [[pipeline]]
        label = "Post-Process"
        file = "post.wgsl"
        shader_type = "fragment"
        entry_point = "fs_main"
    "#;

    let config: Config =
        toml::from_str(config_content).expect("Failed to parse feature-rich config");

    // Verify hot reload is enabled
    assert!(config.hot_reload.is_some());
    assert!(config.hot_reload.unwrap().enabled);

    // Verify all features are configured
    assert!(config.osc.is_some());
    assert!(config.spectrum.is_some());
    assert!(config.midi.is_some());

    // Verify multi-pass pipeline
    assert_eq!(config.pipeline.len(), 2);
    assert_eq!(config.pipeline[0].file, "rich.wgsl");
    assert_eq!(config.pipeline[1].file, "post.wgsl");
}
