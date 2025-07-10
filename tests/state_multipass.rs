use shekere::{Config, State};
use std::path::Path;
use winit::{event_loop::EventLoop, window::WindowBuilder};

#[cfg(test)]
mod tests {
    use super::*;

    // This test will initially fail because State doesn't have multipass support yet
    #[tokio::test]
    async fn test_state_creation_with_multipass_config() {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_title("Test Window")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
            .build(&event_loop)
            .unwrap();

        // Create a multi-pass configuration
        let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Main Scene"
entry_point = "fs_main"
file = "basic.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Blur Effect"
entry_point = "fs_main"
file = "blur.wgsl"
"#;

        let config: Config = toml::from_str(config_content).unwrap();
        let conf_dir = Path::new("tests/shaders");

        // This should work with multipass support
        let state = State::new(&window, &config, conf_dir).await.unwrap();

        // Basic validation that State was created successfully
        assert_eq!(state.size().width, 800);
        assert_eq!(state.size().height, 600);
    }

    #[tokio::test]
    async fn test_state_creation_with_pingpong_config() {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_title("Test Window")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
            .build(&event_loop)
            .unwrap();

        // Create a ping-pong configuration
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

        let config: Config = toml::from_str(config_content).unwrap();
        let conf_dir = Path::new("tests/shaders");

        // This should work with ping-pong support
        let state = State::new(&window, &config, conf_dir).await.unwrap();

        // Basic validation that State was created successfully
        assert_eq!(state.size().width, 800);
        assert_eq!(state.size().height, 600);
    }

    #[tokio::test]
    async fn test_state_creation_with_single_pass_config() {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_title("Test Window")
            .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
            .build(&event_loop)
            .unwrap();

        // Create a single-pass configuration (backward compatibility)
        let config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Basic Shader"
entry_point = "fs_main"
file = "basic.wgsl"
"#;

        let config: Config = toml::from_str(config_content).unwrap();
        let conf_dir = Path::new("tests/shaders");

        // This should work with backward compatibility
        let state = State::new(&window, &config, conf_dir).await.unwrap();

        // Basic validation that State was created successfully
        assert_eq!(state.size().width, 800);
        assert_eq!(state.size().height, 600);
    }
}
