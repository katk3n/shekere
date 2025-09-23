use shekere_core::Config;
use toml;

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

#[cfg(test)]
mod ping_pong_texture_manager_tests {
    use shekere_core::texture_manager::*;
    use tokio;
    use wgpu::{Device, DeviceDescriptor, Features, Instance, Limits, MemoryHints, Queue};

    async fn create_test_device() -> (Device, Queue) {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: MemoryHints::default(),
                },
                None,
            )
            .await
            .unwrap()
    }

    #[test]
    fn test_ping_pong_texture_creation() {
        if std::env::var("CI").is_ok() {
            return; // Skip GPU tests in CI
        }

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut manager = TextureManager::new(&device, 256, 256);

            // Create ping-pong texture
            let (_view1, _sampler1) = manager.get_or_create_ping_pong_texture(&device, 0);

            // Verify texture was created and stored
            assert!(manager.ping_pong_textures.contains_key(&0));
            let textures = manager.ping_pong_textures.get(&0).unwrap();
            assert_eq!(textures.len(), 2); // Should have buffer A and B

            // Verify initialization tracking
            assert!(!manager.is_ping_pong_texture_initialized(0));
            manager.mark_ping_pong_texture_initialized(0);
            assert!(manager.is_ping_pong_texture_initialized(0));
        });
    }

    #[test]
    fn test_ping_pong_buffer_alternation() {
        if std::env::var("CI").is_ok() {
            return; // Skip GPU tests in CI
        }

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut manager = TextureManager::new(&device, 256, 256);

            // Create ping-pong texture
            let _ = manager.get_or_create_ping_pong_texture(&device, 0);

            // Frame 0: should get index 0 for writing, index 1 for reading
            assert_eq!(manager.current_frame, 0);
            let write_index_0 = (manager.current_frame % 2) as usize;

            // Advance frame
            manager.advance_frame();

            // Frame 1: should get index 1 for writing, index 0 for reading
            assert_eq!(manager.current_frame, 1);
            let write_index_1 = (manager.current_frame % 2) as usize;

            // The write indices should be different between frames
            assert_ne!(write_index_0, write_index_1);
        });
    }

    #[test]
    fn test_ping_pong_vs_persistent_texture_differences() {
        if std::env::var("CI").is_ok() {
            return; // Skip GPU tests in CI
        }

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut manager = TextureManager::new(&device, 256, 256);

            // Create both types
            let _ = manager.get_or_create_ping_pong_texture(&device, 0);
            let _ = manager.get_or_create_persistent_texture(&device, 1);

            // Both should have double-buffering
            assert!(manager.ping_pong_textures.contains_key(&0));
            assert!(manager.persistent_textures.contains_key(&1));

            // Both should have separate initialization tracking
            assert!(!manager.is_ping_pong_texture_initialized(0));
            assert!(!manager.is_persistent_texture_initialized(1));

            manager.mark_ping_pong_texture_initialized(0);
            manager.mark_persistent_texture_initialized(1);

            assert!(manager.is_ping_pong_texture_initialized(0));
            assert!(manager.is_persistent_texture_initialized(1));
        });
    }

    #[test]
    fn test_ping_pong_texture_clear() {
        if std::env::var("CI").is_ok() {
            return; // Skip GPU tests in CI
        }

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let (device, _queue) = create_test_device().await;
            let mut manager = TextureManager::new(&device, 256, 256);

            // Create and initialize
            let _ = manager.get_or_create_ping_pong_texture(&device, 0);
            manager.mark_ping_pong_texture_initialized(0);
            assert!(manager.is_ping_pong_texture_initialized(0));

            // Clear all textures
            manager.clear_all_textures();

            // Should be reset
            assert!(!manager.ping_pong_textures.contains_key(&0));
            assert!(!manager.is_ping_pong_texture_initialized(0));
            assert_eq!(manager.current_frame, 0);
        });
    }
}

#[cfg(test)]
mod ping_pong_integration_tests {
    use shekere_core::Config;
    use shekere_core::texture_manager::TextureType;
    use toml;

    #[test]
    fn test_determine_texture_type_ping_pong() {
        let config_content = r#"
[window]
width = 512
height = 512

[[pipeline]]
shader_type = "fragment"
label = "Regular Pass"
entry_point = "fs_main"
file = "regular.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Ping Pong Pass"
entry_point = "fs_main"
file = "pingpong.wgsl"
ping_pong = true

[[pipeline]]
shader_type = "fragment"
label = "Persistent Pass"
entry_point = "fs_main"
file = "persistent.wgsl"
persistent = true
"#;

        let config: Config = toml::from_str(config_content).unwrap();

        // Mock the determine_texture_type logic
        fn mock_determine_texture_type(config: &Config, pass_index: usize) -> TextureType {
            if let Some(shader_config) = config.pipeline.get(pass_index) {
                if shader_config.ping_pong.unwrap_or(false) {
                    TextureType::PingPong
                } else if shader_config.persistent.unwrap_or(false) {
                    TextureType::Persistent
                } else {
                    TextureType::Intermediate
                }
            } else {
                TextureType::Intermediate
            }
        }

        assert_eq!(
            mock_determine_texture_type(&config, 0),
            TextureType::Intermediate
        );
        assert_eq!(
            mock_determine_texture_type(&config, 1),
            TextureType::PingPong
        );
        assert_eq!(
            mock_determine_texture_type(&config, 2),
            TextureType::Persistent
        );
    }

    #[test]
    fn test_ping_pong_config_validation_edge_cases() {
        // Test empty ping_pong field
        let config1 = r#"
[window]
width = 512
height = 512

[[pipeline]]
shader_type = "fragment"
label = "Test"
entry_point = "fs_main"
file = "test.wgsl"
"#;
        let config1: Config = toml::from_str(config1).unwrap();
        assert!(!config1.pipeline[0].ping_pong.unwrap_or(false));

        // Test explicit false
        let config2 = r#"
[window]
width = 512
height = 512

[[pipeline]]
shader_type = "fragment"
label = "Test"
entry_point = "fs_main"
file = "test.wgsl"
ping_pong = false
"#;
        let config2: Config = toml::from_str(config2).unwrap();
        assert!(!config2.pipeline[0].ping_pong.unwrap_or(false));
    }
}
