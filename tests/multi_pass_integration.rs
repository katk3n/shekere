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

// This test verifies that the MultiPassPipeline structure is correctly created
// Note: Full GPU testing requires a graphics context, which isn't available in unit tests
#[cfg(test)]
mod gpu_tests {
    use super::*;
    use shekere::pipeline::MultiPassPipeline;
    use shekere::texture_manager::TextureManager;
    use std::path::PathBuf;
    use wgpu::{Device, Queue};

    async fn create_test_device() -> (Device, Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device")
    }

    #[tokio::test]
    async fn test_multi_pass_pipeline_creation() {
        let (device, _queue) = create_test_device().await;
        let bind_group_layouts = vec![];

        let shader_configs = vec![
            ShaderConfig {
                shader_type: "fragment".to_string(),
                label: "Pass 1".to_string(),
                entry_point: "fs_main".to_string(),
                file: "tests/shaders/basic.wgsl".to_string(),
                ping_pong: None,
                persistent: None,
            },
            ShaderConfig {
                shader_type: "fragment".to_string(),
                label: "Pass 2".to_string(),
                entry_point: "fs_main".to_string(),
                file: "tests/shaders/blur.wgsl".to_string(),
                ping_pong: None,
                persistent: None,
            },
        ];

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: 800,
            height: 600,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let multi_pass_pipeline = MultiPassPipeline::new(
            &device,
            &PathBuf::from("tests"),
            &shader_configs,
            &surface_config,
            &bind_group_layouts,
        );

        assert_eq!(multi_pass_pipeline.pipeline_count(), 2);
        assert!(multi_pass_pipeline.is_multi_pass());
        assert!(multi_pass_pipeline.texture_bind_group_layout.is_some());
    }

    #[tokio::test]
    async fn test_texture_manager_intermediate_textures() {
        let (device, _queue) = create_test_device().await;
        let mut texture_manager = TextureManager::new(&device, 800, 600);

        // Create intermediate textures
        let _ = texture_manager.get_or_create_intermediate_texture(&device, 0);
        let _ = texture_manager.get_or_create_intermediate_texture(&device, 1);

        // Verify render targets are available
        assert!(texture_manager.get_intermediate_render_target(0).is_some());
        assert!(texture_manager.get_intermediate_render_target(1).is_some());
        assert!(texture_manager.get_intermediate_render_target(2).is_none());
    }
}
