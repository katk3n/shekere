use shekere_core::Config;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureFormat};

/// Creates a WebGPU device and queue for testing purposes
pub fn create_test_device_and_queue() -> (Device, Queue) {
    pollster::block_on(async {
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
            .unwrap();

        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap()
    })
}

/// Mock surface configuration for testing
pub fn create_mock_surface_config() -> SurfaceConfiguration {
    SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: TextureFormat::Rgba8Unorm,
        width: 800,
        height: 600,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}

/// Creates a simple test configuration
pub fn create_test_config() -> Config {
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
    toml::from_str(config_content).unwrap()
}

/// Creates a multi-pass test configuration
pub fn create_multipass_test_config() -> Config {
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Pass 1"
entry_point = "fs_main"
file = "pass1.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Pass 2"
entry_point = "fs_main"
file = "pass2.wgsl"
"#;
    toml::from_str(config_content).unwrap()
}

/// Creates a test configuration with ping-pong textures
pub fn create_ping_pong_test_config() -> Config {
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Game of Life"
entry_point = "fs_main"
file = "life.wgsl"
ping_pong = true
"#;
    toml::from_str(config_content).unwrap()
}

/// Creates a test configuration with persistent textures
pub fn create_persistent_test_config() -> Config {
    let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Persistent Effect"
entry_point = "fs_main"
file = "persistent.wgsl"
persistent = true
"#;
    toml::from_str(config_content).unwrap()
}

/// Creates a temporary config directory for testing
pub fn create_test_config_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
