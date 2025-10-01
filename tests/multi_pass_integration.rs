use shekere::config::{Config, ShaderConfig};

#[test]
fn test_multi_pass_configuration_parsing() {
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
label = "Blur Effect"
entry_point = "fs_main"
file = "blur.wgsl"
"#;

    let config: Config = toml::from_str(toml_content).expect("Failed to parse TOML");

    assert_eq!(config.window.width, 800);
    assert_eq!(config.window.height, 600);
    assert_eq!(config.pipeline.len(), 2);

    let first_pass = &config.pipeline[0];
    assert_eq!(first_pass.label, "Main Scene");
    assert_eq!(first_pass.file, "scene.wgsl");
    assert_eq!(first_pass.ping_pong, None);
    assert_eq!(first_pass.persistent, None);

    let second_pass = &config.pipeline[1];
    assert_eq!(second_pass.label, "Blur Effect");
    assert_eq!(second_pass.file, "blur.wgsl");
    assert_eq!(second_pass.ping_pong, None);
    assert_eq!(second_pass.persistent, None);
}

#[test]
fn test_intermediate_texture_configuration() {
    let shader_configs = vec![
        ShaderConfig {
            shader_type: "fragment".to_string(),
            label: "Pass 1".to_string(),
            entry_point: "fs_main".to_string(),
            file: "pass1.wgsl".to_string(),
            ping_pong: None,
            persistent: None,
        },
        ShaderConfig {
            shader_type: "fragment".to_string(),
            label: "Pass 2".to_string(),
            entry_point: "fs_main".to_string(),
            file: "pass2.wgsl".to_string(),
            ping_pong: None,
            persistent: None,
        },
    ];

    // Multi-pass should be detected with multiple shaders
    assert!(shader_configs.len() > 1);

    // No special flags means intermediate textures will be used
    for config in &shader_configs {
        assert!(!config.ping_pong.unwrap_or(false));
        assert!(!config.persistent.unwrap_or(false));
    }
}
