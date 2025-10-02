use shekere::Config;

#[test]
fn test_ping_pong_config_parsing() {
    let config_content = r#"
[window]
width = 512
height = 512

[[pipeline]]
shader_type = "fragment"
label = "Game of Life"
entry_point = "fs_main"
file = "life.wgsl"
ping_pong = true
"#;

    let config: Config = toml::from_str(config_content).unwrap();
    assert!(config.pipeline[0].ping_pong.unwrap_or(false));
    assert!(!config.pipeline[0].persistent.unwrap_or(false));
}

#[test]
fn test_ping_pong_conflicting_flags() {
    let config_content = r#"
[window]
width = 512
height = 512

[[pipeline]]
shader_type = "fragment"
label = "Invalid Config"
entry_point = "fs_main"
file = "test.wgsl"
ping_pong = true
persistent = true
"#;

    let config: Config = toml::from_str(config_content).unwrap();
    // The config parsing should succeed, but validation should fail
    let validation_result = config.validate();
    assert!(validation_result.is_err());
    assert!(
        validation_result
            .unwrap_err()
            .contains("cannot both be true")
    );
}

#[test]
fn test_ping_pong_multi_shader_config() {
    let config_content = r#"
[window]
width = 512
height = 512

[[pipeline]]
shader_type = "fragment"
label = "First Pass"
entry_point = "fs_main"
file = "pass1.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Ping Pong Pass"
entry_point = "fs_main"
file = "pingpong.wgsl"
ping_pong = true

[[pipeline]]
shader_type = "fragment"
label = "Final Pass"
entry_point = "fs_main"
file = "final.wgsl"
"#;

    let config: Config = toml::from_str(config_content).unwrap();
    assert!(!config.pipeline[0].ping_pong.unwrap_or(false));
    assert!(config.pipeline[1].ping_pong.unwrap_or(false));
    assert!(!config.pipeline[2].ping_pong.unwrap_or(false));

    let validation_result = config.validate();
    assert!(validation_result.is_ok());
}
