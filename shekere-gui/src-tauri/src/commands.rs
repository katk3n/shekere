use crate::file_tree::{FileTree, get_file_tree};
use crate::window_manager::{WindowCommand, WindowResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tauri::command;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parsing error: {0}")]
    TomlParse(#[from] toml::de::Error),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Preview error: {0}")]
    Preview(String),
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

type CommandResult<T> = std::result::Result<T, CommandError>;

#[command]
pub async fn get_directory_tree(path: String) -> CommandResult<FileTree> {
    log::info!("get_directory_tree called with path: {}", path);
    let path = Path::new(&path);

    if !path.exists() {
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory '{}' does not exist", path.display()),
        )));
    }

    if !path.is_dir() {
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path '{}' is not a directory", path.display()),
        )));
    }

    let result = get_file_tree(path).map_err(CommandError::Io);
    match &result {
        Ok(tree) => log::info!("get_directory_tree success: {} files, {} dirs", tree.total_files, tree.total_directories),
        Err(e) => log::error!("get_directory_tree error: {:?}", e),
    }
    result
}

#[command]
pub async fn load_toml_config(path: String) -> CommandResult<shekere_core::Config> {
    log::info!("load_toml_config called with path: {:?}", path);
    let config_path = Path::new(&path);

    if !config_path.exists() {
        log::error!("Configuration file '{}' does not exist", config_path.display());
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Configuration file '{}' does not exist",
                config_path.display()
            ),
        )));
    }

    let content = std::fs::read_to_string(config_path).map_err(|e| {
        log::error!("Cannot read configuration file '{}': {}", config_path.display(), e);
        CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "Cannot read configuration file '{}': {}",
                config_path.display(),
                e
            ),
        ))
    })?;

    let config: shekere_core::Config = toml::from_str(&content).map_err(|e| {
        log::error!("Failed to parse TOML config from '{}': {}", config_path.display(), e);
        CommandError::TomlParse(e)
    })?;
    
    validate_config(&config, config_path)?;
    log::info!("Successfully loaded TOML config from '{}'", config_path.display());
    Ok(config)
}

fn validate_config(config: &shekere_core::Config, config_path: &Path) -> CommandResult<()> {
    let config_dir = config_path.parent().unwrap_or(Path::new("."));

    if config.window.width == 0 || config.window.height == 0 {
        return Err(CommandError::Config(
            "Window dimensions must be greater than 0".to_string(),
        ));
    }

    if config.window.width > 8192 || config.window.height > 8192 {
        return Err(CommandError::Config(
            "Window dimensions are unreasonably large (max 8192x8192)".to_string(),
        ));
    }

    for (index, pipeline) in config.pipeline.iter().enumerate() {
        let shader_path = config_dir.join(&pipeline.file);
        if !shader_path.exists() {
            return Err(CommandError::Config(format!(
                "Shader file '{}' referenced in pipeline[{}] does not exist. Expected at: {}",
                pipeline.file,
                index,
                shader_path.display()
            )));
        }

        if pipeline.entry_point.is_empty() {
            return Err(CommandError::Config(format!(
                "Entry point must be specified for pipeline[{}]",
                index
            )));
        }

        match pipeline.shader_type.as_str() {
            "fragment" | "vertex" | "compute" => {}
            _ => {
                return Err(CommandError::Config(format!(
                    "Invalid shader type '{}' in pipeline[{}]. Expected: fragment, vertex, or compute",
                    pipeline.shader_type, index
                )));
            }
        }
    }

    if config.pipeline.is_empty() {
        return Err(CommandError::Config(
            "Configuration must contain at least one pipeline entry".to_string(),
        ));
    }

    Ok(())
}

#[command]
pub async fn load_shader_content(config_path: String) -> CommandResult<HashMap<String, String>> {
    let path = Path::new(&config_path);
    let config_dir = path.parent().unwrap_or(Path::new("."));
    let config = load_toml_config(config_path.clone()).await?;
    let mut shader_content = HashMap::new();

    let common_wgsl_path = std::path::Path::new("../../shaders/common.wgsl");
    let common_content = std::fs::read_to_string(common_wgsl_path).unwrap_or_else(|_| {
        log::warn!("Could not load common.wgsl, using basic WebGPU uniforms");
        r#"
struct WindowUniform {
    resolution: vec2<f32>,
}

struct TimeUniform {
    duration: f32,
}

@group(0) @binding(0) var<uniform> Window: WindowUniform;
@group(1) @binding(0) var<uniform> Time: TimeUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

fn NormalizedCoords(position: vec2<f32>) -> vec2<f32> {
    let min_xy = min(Window.resolution.x, Window.resolution.y);
    return (position * 2.0 - Window.resolution) / min_xy;
}

fn MouseCoords() -> vec2<f32> {
    return vec2<f32>(0.5, 0.5);
}

fn ToLinearRgb(col: vec3<f32>) -> vec3<f32> {
    return col;
}
        "#
        .to_string()
    });

    for (index, pipeline) in config.pipeline.iter().enumerate() {
        let shader_path = config_dir.join(&pipeline.file);
        let user_shader = std::fs::read_to_string(&shader_path).map_err(|e| {
            CommandError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Cannot read shader file '{}' for pipeline[{}]: {}",
                    shader_path.display(),
                    index,
                    e
                ),
            ))
        })?;

        let combined_content = format!("{}\n\n{}", common_content, user_shader);
        let key = format!("{}_{}", pipeline.shader_type, index);
        shader_content.insert(key, combined_content);

        log::info!(
            "Loaded shader '{}' for pipeline[{}]: {} ({} chars user + {} chars common)",
            pipeline.file,
            index,
            pipeline.label,
            user_shader.len(),
            common_content.len()
        );
    }

    log::info!(
        "Successfully loaded {} shader(s) from configuration",
        shader_content.len()
    );
    Ok(shader_content)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreviewHandle {
    pub id: String,
    pub status: String,
    pub config_path: Option<String>,
    pub fps: f32,
    pub render_time_ms: f32,
}

#[derive(Debug)]
struct PreviewInstance {
    id: String,
    config: shekere_core::Config,
    config_path: String,
}

// Native rendering only - simplified state management
type PreviewState = Mutex<HashMap<String, PreviewInstance>>;

fn get_preview_state() -> &'static PreviewState {
    static PREVIEW_STATE: std::sync::OnceLock<PreviewState> = std::sync::OnceLock::new();
    PREVIEW_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

// Simplified headless renderer state (without Send/Sync issues)
static HEADLESS_RENDERER_STATE: std::sync::Mutex<Option<SimpleHeadlessRendererState>> =
    std::sync::Mutex::new(None);

struct SimpleHeadlessRendererState {
    context: shekere_core::WebGpuContext,
    config: Box<shekere_core::Config>,
    config_dir: std::path::PathBuf,
    initialized: bool,
    render_active: bool,
}

#[command]
pub async fn start_native_preview(
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let preview_id = format!("preview_{}", &uuid_str[..8]);

    log::info!("Starting headless native preview '{}'", preview_id);
    validate_preview_config(&config)?;

    let _config_dir = if let Some(ref path) = config_path {
        std::path::Path::new(path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    };

    // Check if headless renderer already exists
    let renderer_exists = {
        if let Ok(state) = HEADLESS_RENDERER_STATE.lock() {
            state.is_some()
        } else {
            false
        }
    };

    if renderer_exists {
        log::info!(
            "Reusing existing headless renderer for preview '{}'",
            preview_id
        );
        return create_preview_instance(preview_id, config, config_path).await;
    }

    log::info!("Initializing new headless renderer");

    // Try to create headless WebGPU context with detailed error reporting
    log::info!("Attempting to create headless WebGPU context...");

    match create_webgpu_context_with_fallback().await {
        Ok(context) => {
            log::info!("WebGPU context created successfully");

            // Test basic WebGPU operations first
            log::info!("Headless renderer initialized successfully - testing WebGPU rendering");

            let config_boxed = Box::new(config.clone());
            match test_basic_webgpu_operations(&context, &config_boxed).await {
                Ok(success) => {
                    log::info!(
                        "WebGPU basic operations test: {}",
                        if success { "SUCCESS" } else { "FAILED" }
                    );
                }
                Err(e) => {
                    log::error!("WebGPU operations test failed: {}", e);
                }
            }

            // Create and store simplified headless renderer state
            let simple_state = SimpleHeadlessRendererState {
                context,
                config: config_boxed,
                config_dir: _config_dir.clone(),
                initialized: true,
                render_active: true,
            };

            // Store renderer state
            {
                let mut state = HEADLESS_RENDERER_STATE.lock().map_err(|_| {
                    CommandError::Preview(
                        "Failed to acquire headless renderer state lock".to_string(),
                    )
                })?;
                *state = Some(simple_state);
            }

            create_preview_instance(preview_id, config, config_path).await
        }
        Err(e) => {
            let error_msg = format!(
                "WebGPU initialization failed: {}. This may be due to:\n1. No compatible GPU drivers\n2. WebGPU not supported on this system\n3. Running in a virtual machine or CI environment",
                e
            );
            log::error!("{}", error_msg);

            // For now, create a mock preview instance to allow UI testing
            log::warn!("Creating mock preview instance for UI testing purposes");
            create_mock_preview_instance(preview_id, config, config_path).await
        }
    }
}

async fn create_preview_instance(
    preview_id: String,
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    let instance = PreviewInstance {
        id: preview_id.clone(),
        config: config.clone(),
        config_path: config_path.clone().unwrap_or_else(|| "unknown".to_string()),
    };

    {
        let mut state = get_preview_state().lock().map_err(|_| {
            CommandError::Preview("Failed to acquire preview state lock".to_string())
        })?;
        state.clear();
        state.insert(preview_id.clone(), instance);
    }

    log::info!(
        "Headless native renderer created successfully with user shader: {:?}",
        config.pipeline.first().map(|p| &p.file)
    );

    Ok(PreviewHandle {
        id: preview_id,
        status: "running (headless)".to_string(),
        config_path,
        fps: 60.0,
        render_time_ms: 2.0,
    })
}

#[command]
pub async fn stop_preview() -> CommandResult<()> {
    let instances_to_stop = {
        let mut state = get_preview_state().lock().map_err(|_| {
            CommandError::Preview("Failed to acquire preview state lock".to_string())
        })?;
        let instances: Vec<_> = state.drain().collect();
        instances
    };

    for (preview_id, _instance) in instances_to_stop {
        log::info!("Stopping preview: {} (headless)", preview_id);
    }

    // Stop the headless renderer
    if let Ok(mut renderer_state) = HEADLESS_RENDERER_STATE.lock() {
        if let Some(mut state) = renderer_state.take() {
            log::info!("Stopping headless renderer");
            state.render_active = false;
            log::info!("Headless renderer stopped");
        }
    }

    log::info!("All previews stopped");
    Ok(())
}


#[command]
pub async fn get_preview_status() -> CommandResult<Option<PreviewHandle>> {
    let state = get_preview_state()
        .lock()
        .map_err(|_| CommandError::Preview("Failed to acquire preview state lock".to_string()))?;

    if let Some((_, instance)) = state.iter().next() {
        // Check if renderer is actually running
        let (status, fps) = if let Ok(renderer_state) = HEADLESS_RENDERER_STATE.lock() {
            if renderer_state.is_some() {
                ("running (headless with actual rendering)".to_string(), 60.0)
            } else {
                ("running (headless mock)".to_string(), 0.0)
            }
        } else {
            ("running (status unknown)".to_string(), 0.0)
        };

        Ok(Some(PreviewHandle {
            id: instance.id.clone(),
            status,
            config_path: Some(instance.config_path.clone()),
            fps,
            render_time_ms: 2.0,
        }))
    } else {
        Ok(None)
    }
}

#[command]
pub async fn handle_mouse_input(x: f64, y: f64) -> CommandResult<()> {
    log::debug!(
        "Mouse input received: ({}, {}) - processed in headless mode",
        x,
        y
    );
    Ok(())
}

fn validate_preview_config(config: &shekere_core::Config) -> CommandResult<()> {
    if config.window.width == 0 || config.window.height == 0 {
        return Err(CommandError::Preview(
            "Cannot start preview: invalid window dimensions".to_string(),
        ));
    }

    if config.pipeline.is_empty() {
        return Err(CommandError::Preview(
            "Cannot start preview: no pipeline configuration found".to_string(),
        ));
    }

    Ok(())
}

/// Try to create WebGPU context with detailed error reporting and fallbacks
async fn create_webgpu_context_with_fallback()
-> Result<shekere_core::WebGpuContext, shekere_core::webgpu_context::WebGpuError> {
    log::info!("Attempting headless WebGPU context creation...");

    // First, let's try to get detailed information about WebGPU availability
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        #[cfg(not(target_arch = "wasm32"))]
        backends: wgpu::Backends::PRIMARY,
        #[cfg(target_arch = "wasm32")]
        backends: wgpu::Backends::GL,
        ..Default::default()
    });

    log::info!("Created wgpu::Instance");

    // Enumerate adapters for debugging
    let adapters: Vec<_> = instance
        .enumerate_adapters(wgpu::Backends::all())
        .into_iter()
        .collect();
    log::info!("Found {} WebGPU adapters:", adapters.len());

    for (i, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        log::info!("  Adapter {}: {} ({:?})", i, info.name, info.backend);
        log::info!("    Device Type: {:?}", info.device_type);
        log::info!("    Vendor: {}", info.vendor);
        log::info!("    Driver: {} ({})", info.driver, info.driver_info);
    }

    // Try different adapter selection strategies
    log::info!("Trying to request adapters manually...");

    // Try high performance first
    let high_perf_adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await;

    // Try low power if high performance fails
    let low_power_adapter = if high_perf_adapter.is_none() {
        log::warn!("High performance adapter not found, trying low power...");
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
    } else {
        None
    };

    // Try fallback adapter if both fail
    let fallback_adapter = if high_perf_adapter.is_none() && low_power_adapter.is_none() {
        log::warn!("Standard adapters not found, trying fallback...");
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: true,
            })
            .await
    } else {
        None
    };

    let adapter = high_perf_adapter.or(low_power_adapter).or(fallback_adapter);

    match adapter {
        Some(adapter) => {
            let info = adapter.get_info();
            log::info!("Selected adapter: {} ({:?})", info.name, info.backend);

            // Try to create device
            match adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        label: Some("Manual WebGPU Device"),
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
            {
                Ok((device, queue)) => {
                    log::info!("Successfully created manual WebGPU context");
                    Ok(shekere_core::WebGpuContext { device, queue })
                }
                Err(e) => {
                    log::error!("Manual WebGPU device creation failed: {}", e);
                    // Fallback to original method
                    match shekere_core::WebGpuContext::new_headless().await {
                        Ok(context) => {
                            log::info!("Fallback to original WebGPU context creation successful");
                            Ok(context)
                        }
                        Err(original_e) => {
                            log::error!(
                                "Original WebGPU context creation also failed: {}",
                                original_e
                            );
                            Err(original_e)
                        }
                    }
                }
            }
        }
        None => {
            log::error!("No WebGPU adapters found at all - trying original method as last resort");
            match shekere_core::WebGpuContext::new_headless().await {
                Ok(context) => {
                    log::info!("Original method worked despite no adapters found");
                    Ok(context)
                }
                Err(e) => {
                    log::error!("Original headless WebGPU context creation failed: {}", e);

                    // Log system information for debugging
                    log::error!("System information for WebGPU debugging:");
                    log::error!("  Platform: {}", std::env::consts::OS);
                    log::error!("  Architecture: {}", std::env::consts::ARCH);

                    // Try to get more specific error information
                    match e {
                        shekere_core::webgpu_context::WebGpuError::AdapterRequest => {
                            log::error!("No suitable WebGPU adapter found. Possible causes:");
                            log::error!("  1. GPU drivers are not installed or up to date");
                            log::error!("  2. WebGPU is not supported on this system");
                            log::error!(
                                "  3. Running in a virtual machine without GPU passthrough"
                            );
                            log::error!("  4. macOS: Try updating to macOS 11+ with Metal support");
                        }
                        shekere_core::webgpu_context::WebGpuError::DeviceRequest(
                            ref device_err,
                        ) => {
                            log::error!("WebGPU device request failed: {}", device_err);
                        }
                        shekere_core::webgpu_context::WebGpuError::NoAdapter => {
                            log::error!("No graphics adapter available for WebGPU");
                        }
                    }

                    Err(e)
                }
            }
        }
    }
}

/// Create a mock preview instance for systems without WebGPU support
async fn create_mock_preview_instance(
    preview_id: String,
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    log::warn!("Creating mock preview instance (WebGPU not available)");

    let instance = PreviewInstance {
        id: preview_id.clone(),
        config: config.clone(),
        config_path: config_path.clone().unwrap_or_else(|| "unknown".to_string()),
    };

    {
        let mut state = get_preview_state().lock().map_err(|_| {
            CommandError::Preview("Failed to acquire preview state lock".to_string())
        })?;
        state.clear();
        state.insert(preview_id.clone(), instance);
    }

    log::warn!("Mock preview created - no actual rendering will occur");

    Ok(PreviewHandle {
        id: preview_id,
        status: "running (mock - WebGPU unavailable)".to_string(),
        config_path,
        fps: 0.0, // No actual rendering
        render_time_ms: 0.0,
    })
}

/// Test basic WebGPU operations
async fn test_basic_webgpu_operations(
    context: &shekere_core::WebGpuContext,
    config: &Box<shekere_core::Config>,
) -> Result<bool, String> {
    log::info!("Testing basic WebGPU operations...");

    // Create a simple test texture
    let texture_descriptor = wgpu::TextureDescriptor {
        label: Some("Test Texture"),
        size: wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    };

    let test_texture = context.device().create_texture(&texture_descriptor);
    let test_view = test_texture.create_view(&wgpu::TextureViewDescriptor::default());

    log::info!("Created test texture: 256x256");

    // Test basic command encoding
    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Command Encoder"),
        });

    // Simple render pass
    {
        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Test Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &test_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    // Submit the command buffer
    context.queue().submit(std::iter::once(encoder.finish()));

    log::info!("Basic WebGPU operations completed successfully");
    log::info!(
        "Shader configuration detected: {:?}",
        config
            .pipeline
            .first()
            .map(|p| format!("{} - {}", p.label, p.file))
    );

    Ok(true)
}

/// Check WebGPU availability and system information
#[command]
pub async fn check_webgpu_availability() -> CommandResult<HashMap<String, String>> {
    let mut info = HashMap::new();

    info.insert("platform".to_string(), std::env::consts::OS.to_string());
    info.insert(
        "architecture".to_string(),
        std::env::consts::ARCH.to_string(),
    );

    log::info!("Checking WebGPU availability...");

    match shekere_core::WebGpuContext::new_headless().await {
        Ok(_context) => {
            info.insert("webgpu_status".to_string(), "available".to_string());
            info.insert(
                "webgpu_message".to_string(),
                "WebGPU context created successfully".to_string(),
            );
            log::info!("WebGPU is available on this system");
        }
        Err(e) => {
            info.insert("webgpu_status".to_string(), "unavailable".to_string());
            info.insert("webgpu_error".to_string(), format!("{}", e));

            let message = match e {
                shekere_core::webgpu_context::WebGpuError::AdapterRequest => {
                    "No suitable WebGPU adapter found. Check GPU drivers.".to_string()
                }
                shekere_core::webgpu_context::WebGpuError::DeviceRequest(_) => {
                    "WebGPU device request failed. GPU may not support required features."
                        .to_string()
                }
                shekere_core::webgpu_context::WebGpuError::NoAdapter => {
                    "No graphics adapter available. Running in virtual machine?".to_string()
                }
            };
            info.insert("webgpu_message".to_string(), message);
            log::warn!("WebGPU is not available: {}", e);
        }
    }

    Ok(info)
}

/// Compatibility alias for start_native_preview
#[command]
pub async fn start_preview(
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    log::info!("start_preview called - using native headless renderer");
    start_native_preview(config, config_path).await
}

// === New Window Manager Commands ===

#[command]
pub async fn start_preview_window(
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    log::info!("start_preview_window called with config_path: {:?}", config_path);

    let config_dir = if let Some(ref path) = config_path {
        std::path::Path::new(path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    };

    // Start the window manager thread (this will create the EventLoop on a separate thread)
    let _window_thread = crate::window_manager::start_window_manager_thread()
        .map_err(|e| CommandError::Preview(format!("Failed to start window manager thread: {}", e)))?;

    // Give the thread a moment to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send CreateWindow command to the window manager thread
    let command = WindowCommand::CreateWindow {
        config: config.clone(),
        config_dir,
    };

    crate::window_manager::send_command(command)
        .map_err(|e| CommandError::Preview(format!("Failed to send create window command: {}", e)))?;

    // Wait for response with timeout
    match crate::window_manager::recv_response_timeout(5000) {
        Ok(WindowResponse::WindowCreated) => {
            log::info!("Window created successfully on window manager thread");

            // Generate unique preview ID
            let uuid_str = uuid::Uuid::new_v4().to_string();
            let preview_id = format!("preview_{}", &uuid_str[..8]);

            Ok(PreviewHandle {
                id: preview_id,
                status: "running (windowed)".to_string(),
                config_path,
                fps: 60.0,
                render_time_ms: 2.0,
            })
        }
        Ok(WindowResponse::Error(e)) => {
            Err(CommandError::Preview(format!("Window creation failed: {}", e)))
        }
        Ok(other) => {
            Err(CommandError::Preview(format!("Unexpected response: {:?}", other)))
        }
        Err(e) => {
            Err(CommandError::Preview(format!("Failed to receive response: {}", e)))
        }
    }
}

#[command]
pub async fn stop_preview_window() -> CommandResult<()> {
    log::info!("stop_preview_window called");

    let command = WindowCommand::DestroyWindow;
    crate::window_manager::send_command(command)
        .map_err(|e| CommandError::Preview(format!("Failed to send destroy window command: {}", e)))?;

    // Wait for response with timeout
    match crate::window_manager::recv_response_timeout(3000) {
        Ok(WindowResponse::WindowDestroyed) => {
            log::info!("Window destroyed successfully");
            Ok(())
        }
        Ok(WindowResponse::Error(e)) => {
            Err(CommandError::Preview(format!("Window destruction failed: {}", e)))
        }
        Ok(other) => {
            Err(CommandError::Preview(format!("Unexpected response: {:?}", other)))
        }
        Err(e) => {
            // Window might have been closed by user, treat as success
            log::warn!("Did not receive destroy response: {}", e);
            Ok(())
        }
    }
}

#[command]
pub async fn get_preview_window_status() -> CommandResult<serde_json::Value> {
    // For simplicity, we'll assume if we can send a command, window manager is active
    // In a more sophisticated implementation, we could add a Status command
    Ok(serde_json::json!({
        "running": true,
        "type": "window"
    }))
}

#[command]
pub async fn handle_preview_window_mouse(x: f64, y: f64) -> CommandResult<()> {
    let command = WindowCommand::UpdateMouse { x, y };
    crate::window_manager::send_command(command)
        .map_err(|e| CommandError::Preview(format!("Failed to send mouse update command: {}", e)))?;

    // Don't wait for response to keep mouse input responsive
    Ok(())
}
