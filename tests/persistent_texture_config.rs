use shekere::{Config, ShaderConfig};

#[test]
fn test_persistent_texture_config_parsing() {
    let toml_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Trail Effect"
entry_point = "fs_main"
file = "trail.wgsl"
persistent = true
"#;

    let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

    assert_eq!(config.window.width, 800);
    assert_eq!(config.window.height, 600);
    assert_eq!(config.pipeline.len(), 1);

    let shader = &config.pipeline[0];
    assert_eq!(shader.shader_type, "fragment");
    assert_eq!(shader.label, "Trail Effect");
    assert_eq!(shader.entry_point, "fs_main");
    assert_eq!(shader.file, "trail.wgsl");
    assert_eq!(shader.persistent, Some(true));
    assert_eq!(shader.ping_pong, None);
}

#[test]
fn test_mixed_pipeline_with_persistent() {
    let toml_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Main Scene"
entry_point = "fs_main"
file = "scene.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Persistent Effect"
entry_point = "fs_main"
file = "persistent.wgsl"
persistent = true

[[pipeline]]
shader_type = "fragment"
label = "Final Output"
entry_point = "fs_main"
file = "output.wgsl"
"#;

    let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

    assert_eq!(config.pipeline.len(), 3);

    // First shader: normal
    let shader1 = &config.pipeline[0];
    assert_eq!(shader1.persistent, None);
    assert_eq!(shader1.ping_pong, None);

    // Second shader: persistent
    let shader2 = &config.pipeline[1];
    assert_eq!(shader2.persistent, Some(true));
    assert_eq!(shader2.ping_pong, None);

    // Third shader: normal
    let shader3 = &config.pipeline[2];
    assert_eq!(shader3.persistent, None);
    assert_eq!(shader3.ping_pong, None);
}

#[test]
fn test_persistent_texture_validation() {
    // Test that we can create configs with persistent textures
    let valid_persistent = ShaderConfig {
        shader_type: "fragment".to_string(),
        label: "Persistent Shader".to_string(),
        entry_point: "fs_main".to_string(),
        file: "persistent.wgsl".to_string(),
        ping_pong: None,
        persistent: Some(true),
    };

    // This should be valid (no conflicts)
    assert!(valid_persistent.ping_pong.is_none());
    assert_eq!(valid_persistent.persistent, Some(true));
}

#[test]
fn test_persistent_false_explicit() {
    let toml_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Normal Shader"
entry_point = "fs_main"
file = "normal.wgsl"
persistent = false
"#;

    let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

    let shader = &config.pipeline[0];
    assert_eq!(shader.persistent, Some(false));
}
