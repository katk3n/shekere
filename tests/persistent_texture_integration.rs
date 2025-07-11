use shekere::Config;

// Helper function to create a basic test config with persistent texture
fn create_persistent_config() -> Config {
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

    toml::from_str(toml_content).expect("Failed to parse test config")
}

// Helper function to create a mixed multi-pass config with persistent texture
fn create_mixed_multipass_config() -> Config {
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

    toml::from_str(toml_content).expect("Failed to parse test config")
}

#[test]
fn test_persistent_texture_config_validation() {
    let config = create_persistent_config();

    assert_eq!(config.pipeline.len(), 1);
    let shader = &config.pipeline[0];
    assert_eq!(shader.persistent, Some(true));
    assert_eq!(shader.ping_pong, None);
    assert_eq!(shader.label, "Trail Effect");
}

#[test]
fn test_mixed_multipass_persistent_config() {
    let config = create_mixed_multipass_config();

    assert_eq!(config.pipeline.len(), 3);

    // First shader: normal
    assert_eq!(config.pipeline[0].persistent, None);

    // Second shader: persistent
    assert_eq!(config.pipeline[1].persistent, Some(true));

    // Third shader: normal
    assert_eq!(config.pipeline[2].persistent, None);
}

#[test]
fn test_texture_type_determination() {
    // This test validates the logic for determining texture types
    // We can't easily test the actual render method due to GPU requirements,
    // but we can test the configuration parsing that drives it

    let config = create_mixed_multipass_config();

    // Test determining texture type from config
    for (i, shader) in config.pipeline.iter().enumerate() {
        let has_ping_pong = shader.ping_pong.unwrap_or(false);
        let has_persistent = shader.persistent.unwrap_or(false);

        match i {
            0 => {
                assert!(
                    !has_ping_pong && !has_persistent,
                    "First shader should be intermediate"
                );
            }
            1 => {
                assert!(
                    !has_ping_pong && has_persistent,
                    "Second shader should be persistent"
                );
            }
            2 => {
                assert!(
                    !has_ping_pong && !has_persistent,
                    "Third shader should be intermediate"
                );
            }
            _ => panic!("Unexpected shader index"),
        }
    }
}

// Test that ensures no conflicts between ping_pong and persistent flags
#[test]
fn test_no_texture_flag_conflicts() {
    let configs = [create_persistent_config(), create_mixed_multipass_config()];

    for config in configs {
        for shader in &config.pipeline {
            let has_ping_pong = shader.ping_pong.unwrap_or(false);
            let has_persistent = shader.persistent.unwrap_or(false);

            // Ensure no conflicts (this would be caught by validation in the future)
            assert!(
                !(has_ping_pong && has_persistent),
                "Shader cannot have both ping_pong and persistent flags"
            );
        }
    }
}

// Test intermediate texture naming with persistent textures
#[test]
fn test_texture_configuration_parsing() {
    let config = create_mixed_multipass_config();

    // Verify that the configuration is properly structured for rendering
    assert!(
        config.pipeline.len() > 1,
        "Multi-pass config should have multiple shaders"
    );

    // Check that at least one shader has persistent flag
    let has_persistent = config
        .pipeline
        .iter()
        .any(|shader| shader.persistent.unwrap_or(false));
    assert!(
        has_persistent,
        "Config should contain at least one persistent shader"
    );

    // Verify that the persistent shader is not the last one (for proper testing)
    let last_index = config.pipeline.len() - 1;
    let last_shader = &config.pipeline[last_index];
    assert!(
        !last_shader.persistent.unwrap_or(false),
        "Last shader should not be persistent for testing purposes"
    );
}
