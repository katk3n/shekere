use clap::Parser;
use shekere_core::{Config, Renderer, WebGpuContext};
use std::panic;
use std::path::Path;
use std::process;
use std::rc::Rc;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

#[derive(Debug, Parser)]
#[command(
    name = "shekere-cli",
    version = env!("CARGO_PKG_VERSION"),
    about = "shekere - Creative coding tool for real-time visual effects with WebGPU shaders and audio integration",
    long_about = "shekere is a creative coding tool that combines WebGPU-based fragment shaders with audio integration (OSC and spectrum analysis). It creates real-time visual effects driven by sound and user interaction."
)]
struct Args {
    /// Path to the TOML configuration file
    #[arg(
        value_name = "FILE",
        help = "TOML configuration file specifying shaders, audio settings, and window properties"
    )]
    config_file: String,
}

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Validate and process the configuration file
    if let Err(exit_code) = run_with_error_handling(&args.config_file) {
        process::exit(exit_code);
    }
}

fn run_with_error_handling(config_path: &str) -> Result<(), i32> {
    // Check if config file exists
    let config_file_path = Path::new(config_path);
    if !config_file_path.exists() {
        eprintln!(
            "Error: Configuration file '{}' does not exist.",
            config_path
        );
        eprintln!("Please provide a valid TOML configuration file.");
        eprintln!("Example: shekere-cli examples/basic/basic.toml");
        return Err(1);
    }

    // Check if it's actually a file (not a directory)
    if !config_file_path.is_file() {
        eprintln!("Error: '{}' is not a file.", config_path);
        eprintln!("Please provide a valid TOML configuration file.");
        return Err(1);
    }

    // Read the configuration file
    let conf_str = match std::fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!(
                "Error: Failed to read configuration file '{}'.",
                config_path
            );
            eprintln!("Reason: {}", err);
            return Err(2);
        }
    };

    // Parse the TOML configuration
    let conf: Config = match toml::from_str(&conf_str) {
        Ok(config) => config,
        Err(err) => {
            eprintln!(
                "Error: Failed to parse TOML configuration file '{}'.",
                config_path
            );
            eprintln!("TOML parsing error: {}", err);
            eprintln!();
            eprintln!("Please check your TOML syntax. Common issues:");
            eprintln!("  - Missing closing brackets ]");
            eprintln!("  - Invalid comment syntax (use # not //)");
            eprintln!("  - Incorrect quotation marks");
            eprintln!("  - Missing required fields");
            return Err(3);
        }
    };

    // Get the directory containing the config file
    let conf_dir = match config_file_path.parent() {
        Some(dir) => dir,
        None => {
            eprintln!(
                "Error: Could not determine directory for configuration file '{}'.",
                config_path
            );
            return Err(4);
        }
    };

    // Validate shader file references (basic check)
    if let Err(validation_error) = validate_shader_files(&conf, conf_dir) {
        eprintln!("Error: Configuration validation failed.");
        eprintln!("{}", validation_error);
        return Err(5);
    }

    // Run the application
    match panic::catch_unwind(|| pollster::block_on(run_shekere(&conf, conf_dir))) {
        Ok(_) => {
            println!("shekere completed successfully.");
            Ok(())
        }
        Err(_) => {
            eprintln!("Error: Application crashed during execution.");
            eprintln!("This might be due to:");
            eprintln!("  - Graphics driver issues");
            eprintln!("  - Missing shader files");
            eprintln!("  - Audio device problems");
            eprintln!("  - Invalid shader code");
            Err(6)
        }
    }
}

/// Basic validation of shader file references in the configuration
fn validate_shader_files(config: &Config, base_dir: &Path) -> Result<(), String> {
    // Check if any pipeline configurations reference non-existent shader files
    for pipeline in &config.pipeline {
        let shader_path = base_dir.join(&pipeline.file);
        if !shader_path.exists() {
            return Err(format!(
                "Shader file '{}' referenced in pipeline '{}' does not exist.\n\
                Expected location: {}\n\
                Please ensure the shader file exists or update the configuration.",
                pipeline.file,
                pipeline.label,
                shader_path.display()
            ));
        }
    }
    Ok(())
}

/// Run shekere with window management using the new Renderer API
/// This function was moved from shekere-core and adapted for Phase 2
async fn run_shekere(conf: &Config, conf_dir: &Path) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = Rc::new(WindowBuilder::new()
        .with_title("shekere")
        .with_inner_size(LogicalSize::new(conf.window.width, conf.window.height))
        .build(&event_loop)
        .unwrap());

    // Get window ID before creating surface (to avoid borrowing issues later)
    let window_id = window.id();

    // Create surface and WebGPU context
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        #[cfg(not(target_arch = "wasm32"))]
        backends: wgpu::Backends::PRIMARY,
        #[cfg(target_arch = "wasm32")]
        backends: wgpu::Backends::GL,
        ..Default::default()
    });
    let surface = instance.create_surface(window.as_ref()).unwrap();

    // Get adapter and configure surface using the same instance
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .unwrap();

    // Create WebGPU context from existing device and queue
    let context = WebGpuContext { device, queue };

    // Configure surface
    let size = window.as_ref().inner_size();
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: if !surface_format.is_srgb() {
            vec![surface_format.add_srgb_suffix()]
        } else {
            vec![]
        },
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&context.device, &surface_config);

    // Create renderer using new API
    let mut renderer = Renderer::new(context, conf, conf_dir, size.width, size.height)
        .await
        .expect("Failed to create renderer");

    // Request initial redraw to start animation loop
    window.as_ref().request_redraw();

    let window_clone = window.clone();
    let _ = event_loop.run(move |event, control_flow| {
        match event {
        Event::WindowEvent {
            ref event,
            window_id: event_window_id,
        } if event_window_id == window_id => {
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => control_flow.exit(),
                WindowEvent::Resized(physical_size) => {
                    if physical_size.width > 0 && physical_size.height > 0 {
                        surface_config.width = physical_size.width;
                        surface_config.height = physical_size.height;
                        surface.configure(renderer.get_device(), &surface_config);
                        renderer.update_size(physical_size.width, physical_size.height);
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    renderer.handle_mouse_input(position.x, position.y);
                }
                WindowEvent::RedrawRequested => {
                    renderer.update(0.0); // delta_time not used currently
                    match renderer.render_to_surface(&surface, &surface_config) {
                        Ok(_) => {}
                        Err(shekere_core::RendererError::Surface(
                            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                        )) => {
                            if size.width > 0 && size.height > 0 {
                                surface_config.width = size.width;
                                surface_config.height = size.height;
                                surface.configure(renderer.get_device(), &surface_config);
                                renderer.update_size(size.width, size.height);
                            }
                        }
                        Err(shekere_core::RendererError::Surface(
                            wgpu::SurfaceError::OutOfMemory,
                        )) => {
                            log::error!("OutOfMemory");
                            control_flow.exit();
                        }
                        Err(shekere_core::RendererError::Surface(wgpu::SurfaceError::Timeout)) => {
                            log::warn!("Surface timeout")
                        }
                        Err(e) => {
                            log::error!("Render error: {}", e);
                        }
                    }

                }
                _ => {}
            }
        }
        Event::AboutToWait => {
            // Request redraw on every event loop iteration for smooth animation
            window_clone.request_redraw();
        }
        _ => {}
        }
    });
}
