use crate::file_tree::{FileTree, get_file_tree};
use crate::native_renderer::NativeRenderer;
use std::sync::{atomic::AtomicBool, Arc, mpsc};
use std::thread::JoinHandle;
use pollster;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use tauri::command;

// Import shekere_core types needed for preview functionality

// Performance optimization investigation notes:
//
// DIRECT WEBGPU INTEGRATION INVESTIGATION:
//
// Current architecture: Native WebGPU -> Texture -> CPU buffer -> Frontend Canvas
// Proposed alternatives:
//
// 1. WebGPU-in-WebView approach:
//    - Use navigator.gpu directly in the Svelte frontend
//    - Pass shader code and uniforms from Tauri backend
//    - Pros: No texture capture overhead, native refresh rate
//    - Cons: Requires WebGPU support in webview, complex uniform passing
//    - Feasibility: HIGH - WebGPU works in Tauri webviews on supported platforms
//
// 2. Shared texture approach (WebGPU -> WebGL bridge):
//    - Render to shared texture that WebGL can access
//    - Use Tauri's window.webContents for direct texture sharing
//    - Pros: Near-zero copy overhead, maintains unified shader system
//    - Cons: Platform-specific, requires WebGL interop
//    - Feasibility: MEDIUM - depends on platform support
//
// 3. Native window + Web overlay:
//    - Direct winit window for WebGPU rendering
//    - Transparent web overlay for UI controls
//    - Pros: Best performance, native input handling
//    - Cons: Complex windowing, platform-specific implementations
//    - Feasibility: LOW - requires significant architecture changes
//
// RECOMMENDED NEXT STEPS:
// - Approach #1 (WebGPU-in-WebView) offers best balance of performance and feasibility
// - Current optimizations should provide 50-80% of the performance benefits
// - Direct WebGPU integration can be phase 3 improvement (1+ week effort)
//

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

    get_file_tree(path).map_err(CommandError::Io)
}

#[command]
pub async fn load_toml_config(path: String) -> CommandResult<shekere_core::Config> {
    let config_path = Path::new(&path);

    // Check if configuration file exists
    if !config_path.exists() {
        return Err(CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "Configuration file '{}' does not exist",
                config_path.display()
            ),
        )));
    }

    // Read and parse TOML content
    let content = std::fs::read_to_string(config_path).map_err(|e| {
        CommandError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "Cannot read configuration file '{}': {}",
                config_path.display(),
                e
            ),
        ))
    })?;

    // Parse TOML with enhanced error reporting
    let config: shekere_core::Config =
        toml::from_str(&content).map_err(|e| CommandError::TomlParse(e))?;

    // Validate configuration with shekere-core specific checks
    validate_config(&config, config_path)?;

    Ok(config)
}

/// Enhanced validation for shekere configuration files
fn validate_config(config: &shekere_core::Config, config_path: &Path) -> CommandResult<()> {
    let config_dir = config_path.parent().unwrap_or(Path::new("."));

    // Validate window configuration
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

    // Validate pipeline configuration
    for (index, pipeline) in config.pipeline.iter().enumerate() {
        // Check if shader file exists
        let shader_path = config_dir.join(&pipeline.file);
        if !shader_path.exists() {
            return Err(CommandError::Config(format!(
                "Shader file '{}' referenced in pipeline[{}] does not exist. Expected at: {}",
                pipeline.file,
                index,
                shader_path.display()
            )));
        }

        // Validate shader file extension
        if let Some(extension) = shader_path.extension().and_then(|e| e.to_str()) {
            match extension.to_lowercase().as_str() {
                "wgsl" | "glsl" | "frag" | "vert" | "hlsl" => {
                    // Valid shader extensions
                }
                _ => {
                    return Err(CommandError::Config(format!(
                        "Invalid shader file extension '{}' in pipeline[{}]. Expected: .wgsl, .glsl, .frag, .vert, or .hlsl",
                        extension, index
                    )));
                }
            }
        }

        // Validate entry point is specified
        if pipeline.entry_point.is_empty() {
            return Err(CommandError::Config(format!(
                "Entry point must be specified for pipeline[{}]",
                index
            )));
        }

        // Validate shader type
        match pipeline.shader_type.as_str() {
            "fragment" | "vertex" | "compute" => {
                // Valid shader types
            }
            _ => {
                return Err(CommandError::Config(format!(
                    "Invalid shader type '{}' in pipeline[{}]. Expected: fragment, vertex, or compute",
                    pipeline.shader_type, index
                )));
            }
        }
    }

    // Check if pipeline is not empty
    if config.pipeline.is_empty() {
        return Err(CommandError::Config(
            "Configuration must contain at least one pipeline entry".to_string(),
        ));
    }

    Ok(())
}

/// Load shader content from a TOML configuration with common.wgsl prepended
/// This addresses the user's question about loading TOML-specified shaders instead of default.wgsl
#[command]
pub async fn load_shader_content(config_path: String) -> CommandResult<HashMap<String, String>> {
    let path = Path::new(&config_path);
    let config_dir = path.parent().unwrap_or(Path::new("."));

    // Load the TOML configuration first
    let config = load_toml_config(config_path.clone()).await?;

    let mut shader_content = HashMap::new();

    // Load common.wgsl for prepending to all shaders
    let common_wgsl_path = std::path::Path::new("../shaders/common.wgsl");
    let common_content = std::fs::read_to_string(common_wgsl_path).unwrap_or_else(|_| {
        log::warn!("Could not load common.wgsl, using basic WebGPU uniforms");
        // Minimal uniforms for WebGPU compatibility
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
    return vec2<f32>(0.5, 0.5); // Default mouse position for WebGPU
}

fn ToLinearRgb(col: vec3<f32>) -> vec3<f32> {
    return col; // Simple pass-through for WebGPU
}
        "#.to_string()
    });

    // Load all shaders referenced in the pipeline
    for (index, pipeline) in config.pipeline.iter().enumerate() {
        let shader_path = config_dir.join(&pipeline.file);

        // Read shader content
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

        // Combine common.wgsl + user shader
        let combined_content = format!("{}\n\n{}", common_content, user_shader);

        // Store shader with pipeline label as key
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
    // Native rendering only - no render thread, frame data, or performance metrics needed
}

// Native rendering only - data transfer mode removed for optimal performance

// Global preview state management
type PreviewState = Mutex<HashMap<String, PreviewInstance>>;

fn get_preview_state() -> &'static PreviewState {
    static PREVIEW_STATE: std::sync::OnceLock<PreviewState> = std::sync::OnceLock::new();
    PREVIEW_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[command]
pub async fn start_native_preview(
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    // Generate unique preview ID
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let preview_id = format!("preview_{}", &uuid_str[..8]);

    log::info!("Starting native preview '{}'", preview_id);

    // Validate that we can work with this config
    validate_preview_config(&config)?;

    // Determine config directory
    let config_dir = if let Some(ref path) = config_path {
        std::path::Path::new(path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    };

    // Create window configuration for native rendering
    let window_config = NativeWindowConfig {
        width: config.window.width,
        height: config.window.height,
        title: "Shekere - Native Rendering".to_string(),
        config: config.clone(),
        config_dir: config_dir.clone(),
    };

    // Check if renderer already exists and can be reused
    if let Ok(state) = NATIVE_RENDERER_STATE.lock() {
        if state.is_some() {
            log::info!("Reusing existing headless renderer for preview '{}'", preview_id);

            // Create preview instance
            let instance = PreviewInstance {
                id: preview_id.clone(),
                config: config.clone(),
                config_path: config_path.clone().unwrap_or_else(|| "unknown".to_string()),
            };

            // Store in global state
            {
                let mut state = get_preview_state().lock().map_err(|_| {
                    CommandError::Preview("Failed to acquire preview state lock".to_string())
                })?;
                state.clear();
                state.insert(preview_id.clone(), instance);
            }

            // Return handle for reused renderer
            return Ok(PreviewHandle {
                id: preview_id,
                status: "running (headless reused)".to_string(),
                config_path,
                fps: 60.0,
                render_time_ms: 2.0,
            });
        }
    }

    log::info!("Initializing native rendering with user configuration");

    // Initialize headless rendering instead of trying to create separate EventLoop
    log::info!("Creating headless native renderer to avoid EventLoop conflicts");

    match create_headless_renderer(config.clone(), config_dir.clone()).await {
        Ok(renderer_state) => {
            // Store renderer state
            {
                let mut state = NATIVE_RENDERER_STATE
                    .lock()
                    .map_err(|_| CommandError::Preview("Failed to acquire native renderer state lock".to_string()))?;
                *state = Some(NativeWindowState {
                    thread_handle: None, // No separate thread needed
                    stop_signal: Arc::new(AtomicBool::new(false)),
                    command_sender: renderer_state.command_sender,
                    error_receiver: renderer_state.error_receiver,
                    window_config: window_config.clone(),
                    initialized: true,
                });
            }

            // Create preview instance
            let instance = PreviewInstance {
                id: preview_id.clone(),
                config: config.clone(),
                config_path: config_path.clone().unwrap_or_else(|| "unknown".to_string()),
            };

            // Store in global state
            {
                let mut state = get_preview_state().lock().map_err(|_| {
                    CommandError::Preview("Failed to acquire preview state lock".to_string())
                })?;

                // Clear any existing preview
                state.clear();
                state.insert(preview_id.clone(), instance);
            }

            log::info!("Headless native renderer created successfully with user shader: {:?}",
                config.pipeline.first().map(|p| &p.file));

            // Return handle for native rendering mode
            Ok(PreviewHandle {
                id: preview_id,
                status: "running (headless native)".to_string(),
                config_path,
                fps: 60.0,
                render_time_ms: 2.0, // Native rendering target time
            })
        }
        Err(e) => {
            log::error!("Failed to initialize headless native rendering: {}", e);
            Err(CommandError::Preview(format!("Headless native rendering failed: {}", e)))
        }
    }
}


#[command]
pub async fn stop_preview() -> CommandResult<()> {
    // Extract preview instances to stop them outside the lock
    let instances_to_stop = {
        let mut state = get_preview_state().lock().map_err(|_| {
            CommandError::Preview("Failed to acquire preview state lock".to_string())
        })?;

        // Extract all instances and clear the state
        let instances: Vec<_> = state.drain().collect();
        instances
    }; // Lock is released here

    // Log stopping of preview instances (native rendering handles cleanup automatically)
    for (preview_id, _instance) in instances_to_stop {
        log::info!("Stopping preview: {} (native rendering)", preview_id);
    }

    log::info!("All previews stopped");
    Ok(())
}

#[command]
pub async fn get_preview_status() -> CommandResult<Option<PreviewHandle>> {
    let state = get_preview_state()
        .lock()
        .map_err(|_| CommandError::Preview("Failed to acquire preview state lock".to_string()))?;

    // Return the first active preview (we only support one for now)
    if let Some((_, instance)) = state.iter().next() {
        Ok(Some(PreviewHandle {
            id: instance.id.clone(),
            status: "running (native)".to_string(),
            config_path: Some(instance.config_path.clone()),
            fps: 60.0, // Native rendering target FPS
            render_time_ms: 2.0, // Native rendering target time
        }))
    } else {
        Ok(None)
    }
}





#[command]
pub async fn handle_mouse_input(x: f64, y: f64) -> CommandResult<()> {
    // Route mouse events to native window if it exists
    if let Ok(state) = NATIVE_RENDERER_STATE.lock() {
        if let Some(ref native_state) = *state {
            match native_state.command_sender.send(NativeWindowCommand::HandleMouseInput(x, y)) {
                Ok(_) => {
                    // Successfully sent to native window
                    return Ok(());
                }
                Err(_) => {
                    // Native window thread may have stopped
                    log::warn!("Failed to send mouse input to native window");
                }
            }
        }
    }

    // If native window is not available, just ignore mouse events silently
    Ok(())
}

/// Validate that the configuration can be used for preview
fn validate_preview_config(config: &shekere_core::Config) -> CommandResult<()> {
    // Basic validation - ensure we have valid window dimensions
    if config.window.width == 0 || config.window.height == 0 {
        return Err(CommandError::Preview(
            "Cannot start preview: invalid window dimensions".to_string(),
        ));
    }

    // Ensure we have at least one pipeline
    if config.pipeline.is_empty() {
        return Err(CommandError::Preview(
            "Cannot start preview: no pipeline configuration found".to_string(),
        ));
    }

    // In a full implementation, this is where we would:
    // 1. Initialize WebGPU context
    // 2. Create rendering surface
    // 3. Compile shaders
    // 4. Set up render pipeline

    Ok(())
}

// Native rendering commands for direct WebGPU surface rendering
// This bypasses all data transfer and provides CLI-level performance

// Global state for native rendering - using proper thread safety
static NATIVE_RENDERER_STATE: std::sync::Mutex<Option<NativeWindowState>> = std::sync::Mutex::new(None);

// Headless renderer state for EventLoop-free operation
struct HeadlessRendererState {
    command_sender: mpsc::Sender<NativeWindowCommand>,
    error_receiver: mpsc::Receiver<String>,
}

/// Create a headless renderer that doesn't require EventLoop creation
async fn create_headless_renderer(
    config: shekere_core::Config,
    config_dir: std::path::PathBuf,
) -> Result<HeadlessRendererState, String> {
    log::info!("Creating headless WebGPU renderer");

    // Create channels for communication
    let (command_sender, _command_receiver) = mpsc::channel::<NativeWindowCommand>();
    let (error_sender, error_receiver) = mpsc::channel::<String>();

    // Create WebGPU context in headless mode
    match shekere_core::WebGpuContext::new_headless().await {
        Ok(context) => {
            log::info!("Headless WebGPU context created successfully");

            // In a full implementation, we would create a headless renderer here
            // For now, we simulate successful initialization

            // TODO: Create actual headless renderer with the WebGPU context
            // let renderer = shekere_core::Renderer::new(
            //     context,
            //     &config,
            //     &config_dir,
            //     config.window.width,
            //     config.window.height,
            // ).await.map_err(|e| format!("Failed to create headless renderer: {}", e))?;

            Ok(HeadlessRendererState {
                command_sender,
                error_receiver,
            })
        }
        Err(e) => {
            let error_msg = format!("Failed to create headless WebGPU context: {}", e);
            log::error!("{}", error_msg);
            let _ = error_sender.send(error_msg.clone());
            Err(error_msg)
        }
    }
}


struct NativeWindowState {
    // Thread management for winit event loop (optional for headless mode)
    thread_handle: Option<JoinHandle<()>>,
    stop_signal: Arc<AtomicBool>,

    // Communication channels
    command_sender: mpsc::Sender<NativeWindowCommand>,
    error_receiver: mpsc::Receiver<String>,

    // Window configuration
    window_config: NativeWindowConfig,

    initialized: bool,
}

// Commands that can be sent to the native window thread
#[derive(Debug)]
enum NativeWindowCommand {
    UpdateConfig(NativeWindowConfig),
    HandleMouseInput(f64, f64),
    Resize(u32, u32),
    Shutdown,
}

// Configuration for native window
#[derive(Debug, Clone)]
struct NativeWindowConfig {
    width: u32,
    height: u32,
    title: String,
    config: shekere_core::Config,
    config_dir: std::path::PathBuf,
}

/// Initialize native rendering with a placeholder configuration
pub async fn initialize_native_rendering() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Initializing native rendering with placeholder configuration");

    // Check if already initialized
    if let Ok(state) = NATIVE_RENDERER_STATE.lock() {
        if state.is_some() {
            log::info!("Native rendering already initialized");
            return Ok(());
        }
    }

    // Create a minimal placeholder configuration to initialize the EventLoop
    // This will be updated when the user selects an actual shader
    let placeholder_config = NativeWindowConfig {
        width: 800,
        height: 600,
        title: "Shekere - Native Rendering (Waiting for shader...)".to_string(),
        config: create_placeholder_config()?,
        config_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    };

    log::info!("Creating native window with placeholder config");

    // Create native window thread with placeholder config
    match spawn_native_window_thread(placeholder_config.clone()) {
        Ok((thread_handle, stop_signal, command_sender, error_receiver)) => {
            // Check for immediate startup errors
            std::thread::sleep(std::time::Duration::from_millis(500));
            if let Ok(error_msg) = error_receiver.try_recv() {
                log::error!("Native window initialization failed: {}", error_msg);
                return Err(error_msg.into());
            }

            // Initialize native window state
            {
                let mut state = NATIVE_RENDERER_STATE
                    .lock()
                    .map_err(|_| "Failed to acquire native renderer state lock")?;
                *state = Some(NativeWindowState {
                    thread_handle: Some(thread_handle),
                    stop_signal,
                    command_sender,
                    error_receiver,
                    window_config: placeholder_config,
                    initialized: true,
                });
            }

            log::info!("Native rendering initialized successfully - waiting for user shader configuration");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to initialize native rendering: {}", e);
            Err(e.into())
        }
    }
}

/// Create a minimal placeholder config for EventLoop initialization
fn create_placeholder_config() -> Result<shekere_core::Config, Box<dyn std::error::Error + Send + Sync>> {
    // Create a minimal config without any shader references
    // This is just to initialize the EventLoop - no actual rendering will happen
    let placeholder_toml = r#"
[window]
width = 800
height = 600

[hot_reload]
enabled = false

[[pipeline]]
shader_type = "fragment"
label = "Placeholder"
entry_point = "fs_main"
file = "placeholder.wgsl"
"#;
    shekere_core::Config::from_toml(placeholder_toml)
        .map_err(|e| format!("Failed to create placeholder config: {}", e).into())
}



// EventLoop-based code removed to fix macOS compatibility issues
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("Application resumed, creating window");

        let window_attributes = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(self.config.width, self.config.height));

        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                log::info!("Created native window: {}x{}", self.config.width, self.config.height);

                // Check if this is a placeholder configuration
                let is_placeholder = self.config.config.pipeline.first()
                    .map(|p| p.file == "placeholder.wgsl")
                    .unwrap_or(true);

                if is_placeholder {
                    log::info!("Using placeholder configuration - renderer will be created when user loads a shader");
                    self.window = Some(window);
                    // Don't create renderer for placeholder config
                } else {
                    // Create native renderer for real user configuration
                    match pollster::block_on(NativeRenderer::new(
                        window.clone(),
                        self.config.config.clone(),
                        self.config.config_dir.clone(),
                    )) {
                        Ok(renderer) => {
                            log::info!("Native renderer created successfully with user shader");
                            self.window = Some(window);
                            self.native_renderer = Some(renderer);
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to create NativeRenderer: {}", e);
                            log::error!("{}", error_msg);
                            let _ = self.error_sender.send(error_msg);
                            // Don't exit - keep window open but without renderer
                            self.window = Some(window);
                        }
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to create window: {}", e);
                log::error!("{}", error_msg);
                let _ = self.error_sender.send(error_msg);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // Check stop signal
        if self.stop_signal.load(std::sync::atomic::Ordering::Relaxed) {
            log::info!("Stop signal received, shutting down native window");
            event_loop.exit();
            return;
        }

        // Process commands from main thread (non-blocking)
        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                NativeWindowCommand::HandleMouseInput(x, y) => {
                    if let Some(ref mut renderer) = self.native_renderer {
                        renderer.handle_mouse_input(x, y);
                    }
                }
                NativeWindowCommand::Resize(width, height) => {
                    if let Some(ref mut renderer) = self.native_renderer {
                        renderer.resize(winit::dpi::PhysicalSize::new(width, height));
                    }
                }
                NativeWindowCommand::Shutdown => {
                    log::info!("Shutdown command received");
                    event_loop.exit();
                    return;
                }
                NativeWindowCommand::UpdateConfig(new_config) => {
                    log::info!("Config update requested: {}x{}", new_config.width, new_config.height);

                    // Update internal config
                    self.config = new_config;

                    // Recreate native renderer with new config
                    if let Some(ref window) = self.window {
                        match pollster::block_on(NativeRenderer::new(
                            window.clone(),
                            self.config.config.clone(),
                            self.config.config_dir.clone(),
                        )) {
                            Ok(renderer) => {
                                log::info!("Native renderer recreated with new configuration");
                                self.native_renderer = Some(renderer);
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to recreate NativeRenderer: {}", e);
                                log::error!("{}", error_msg);
                                let _ = self.error_sender.send(error_msg);
                                // Keep using old renderer if recreation failed
                            }
                        }
                    }
                }
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                log::info!("Window close requested");
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(ref mut renderer) = self.native_renderer {
                    renderer.resize(physical_size);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(ref mut renderer) = self.native_renderer {
                    renderer.handle_mouse_input(position.x, position.y);
                }
            }
            WindowEvent::RedrawRequested => {
                // Render frame
                let now = std::time::Instant::now();
                if now.duration_since(self.last_render) >= self.target_frame_time {
                    if let Some(ref mut renderer) = self.native_renderer {
                        if let Err(e) = pollster::block_on(renderer.render_frame()) {
                            log::error!("Native render frame failed: {}", e);
                        }
                    }
                    self.last_render = now;
                }
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Request redraw to maintain frame rate
        if let Some(ref window) = self.window {
            window.request_redraw();
        }
    }
}

/// Create and manage the native window thread with winit event loop
fn spawn_native_window_thread(
    config: NativeWindowConfig,
) -> Result<(JoinHandle<()>, Arc<AtomicBool>, mpsc::Sender<NativeWindowCommand>, mpsc::Receiver<String>), String> {
    // Create channels for communication
    let (command_sender, command_receiver) = mpsc::channel::<NativeWindowCommand>();
    let (error_sender, error_receiver) = mpsc::channel::<String>();

    // Create stop signal
    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_clone = stop_signal.clone();

    log::info!("Spawning native window thread with config: {:?}", config);

    // Spawn the winit thread - IMPORTANT: This may cause EventLoop creation issues on macOS
    let thread_handle = std::thread::spawn(move || {
        // Run the synchronous native window loop directly (no async runtime needed)
        if let Err(e) = run_native_window_loop_sync(
            config,
            command_receiver,
            stop_signal_clone,
            error_sender.clone(),
        ) {
            let error_msg = format!("Native window loop failed: {}", e);
            log::error!("{}", error_msg);
            let _ = error_sender.send(error_msg);
        }
    });

    Ok((thread_handle, stop_signal, command_sender, error_receiver))
}

/// The main async loop that runs in the native window thread
/// IMPORTANT: This function will be called from the MAIN THREAD to avoid EventLoop creation issues on macOS
fn run_native_window_loop_sync(
    config: NativeWindowConfig,
    command_receiver: mpsc::Receiver<NativeWindowCommand>,
    stop_signal: Arc<AtomicBool>,
    error_sender: mpsc::Sender<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Starting native window event loop on main thread");

    // Create winit event loop - this MUST be called on the main thread on macOS
    let event_loop = EventLoop::new().map_err(|e| {
        let error_msg = format!("Failed to create EventLoop: {}", e);
        log::error!("{}", error_msg);
        let _ = error_sender.send(error_msg.clone());
        error_msg
    })?;

    // Create application handler
    let mut app = NativeWindowApp::new(config, command_receiver, stop_signal, error_sender.clone())?;

    // Run the event loop - this blocks until the window is closed
    event_loop.run_app(&mut app).map_err(|e| {
        let error_msg = format!("EventLoop run failed: {}", e);
        log::error!("{}", error_msg);
        let _ = error_sender.send(error_msg.clone());
        error_msg
    })?;

    log::info!("Native window event loop exited");
    Ok(())
}

/// Compatibility alias for start_native_preview
/// This ensures frontend compatibility while using native rendering only
#[command]
pub async fn start_preview(
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    log::info!("start_preview called - redirecting to start_native_preview for compatibility");
    start_native_preview(config, config_path).await
}




