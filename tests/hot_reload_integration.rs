use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;
use kchfgt::{Config};

#[test]
fn test_config_parsing_with_hot_reload() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp config file");
    
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Test Shader"
entry_point = "fs_main"
file = "test.wgsl"

[hot_reload]
enabled = true
"#;
    
    write!(temp_file, "{}", config_content).expect("Failed to write config");
    
    let config_str = fs::read_to_string(temp_file.path()).expect("Failed to read config file");
    let config = Config::from_toml(&config_str).expect("Failed to parse config");
    
    // Verify hot reload configuration
    assert!(config.hot_reload.is_some(), "Hot reload config should be present");
    assert!(config.hot_reload.as_ref().unwrap().enabled, "Hot reload should be enabled");
    
    // Verify basic validation passes
    assert!(config.validate().is_ok(), "Config should be valid");
}

#[test]
fn test_config_validation_with_hot_reload_disabled() {
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Test Shader"
entry_point = "fs_main"
file = "test.wgsl"

[hot_reload]
enabled = false
"#;
    
    let config = Config::from_toml(config_content).expect("Failed to parse config");
    
    // Verify hot reload configuration
    assert!(config.hot_reload.is_some(), "Hot reload config should be present");
    assert!(!config.hot_reload.as_ref().unwrap().enabled, "Hot reload should be disabled");
    
    // Verify basic validation passes
    assert!(config.validate().is_ok(), "Config should be valid");
}

#[test]
fn test_config_without_hot_reload_section() {
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Test Shader"
entry_point = "fs_main"
file = "test.wgsl"
"#;
    
    let config = Config::from_toml(config_content).expect("Failed to parse config");
    
    // Verify hot reload configuration is None
    assert!(config.hot_reload.is_none(), "Hot reload config should be None when not specified");
    
    // Verify basic validation passes
    assert!(config.validate().is_ok(), "Config should be valid");
}

#[test]
fn test_hot_reload_with_complex_config() {
    let config_content = r#"
[window]
width = 1280
height = 720

[[pipeline]]
shader_type = "fragment"
label = "Main Shader"
entry_point = "fs_main"
file = "complex.wgsl"

[osc]
port = 57120
addr_pattern = "/dirt/play"

[[osc.sound]]
name = "bd"
id = 1

[[osc.sound]]
name = "sd"
id = 2

[spectrum]
min_frequency = 27.0
max_frequency = 2000.0
sampling_rate = 44100

[hot_reload]
enabled = true
"#;
    
    let config = Config::from_toml(config_content).expect("Failed to parse config");
    
    // Verify all features are present and working together
    assert!(config.osc.is_some(), "OSC config should be present");
    assert!(config.spectrum.is_some(), "Spectrum config should be present");
    assert!(config.hot_reload.is_some(), "Hot reload config should be present");
    assert!(config.hot_reload.as_ref().unwrap().enabled, "Hot reload should be enabled");
    
    // Verify validation passes with all features
    assert!(config.validate().is_ok(), "Complex config should be valid");
}