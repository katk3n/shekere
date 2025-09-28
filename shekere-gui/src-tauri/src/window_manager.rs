use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};

use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::EventLoop,
    window::{WindowBuilder, WindowId},
    dpi::LogicalSize,
    keyboard::{PhysicalKey, KeyCode},
};

use wgpu;
use shekere_core::{Config, Renderer, WebGpuContext};

/// Commands that can be sent to control the window
#[derive(Debug, Clone)]
pub enum WindowCommand {
    CreateWindow {
        config: Config,
        config_dir: PathBuf,
    },
    DestroyWindow,
    UpdateMouse { x: f64, y: f64 },
}

/// Response from window operations
#[derive(Debug, Clone)]
pub enum WindowResponse {
    WindowCreated,
    WindowDestroyed,
    MouseUpdated,
    Error(String),
}

/// Global state for window management - runs on main thread
/// Note: EventLoop cannot be stored in global state due to Send/Sync requirements
#[derive(Debug)]
pub struct WindowManager {
    command_queue: Arc<Mutex<Vec<WindowCommand>>>,
    response_queue: Arc<Mutex<Vec<WindowResponse>>>,
}

/// Global state for communication between Tauri commands and WindowManager
/// We only store the communication queues, not the WindowManager itself
static GLOBAL_COMMAND_QUEUE: OnceLock<Arc<Mutex<Vec<WindowCommand>>>> = OnceLock::new();
static GLOBAL_RESPONSE_QUEUE: OnceLock<Arc<Mutex<Vec<WindowResponse>>>> = OnceLock::new();

impl WindowManager {
    /// Create a new WindowManager on the main thread
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            command_queue: Arc::new(Mutex::new(Vec::new())),
            response_queue: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Send a command to the window manager
    pub fn send_command(&self, command: WindowCommand) -> Result<(), String> {
        let mut queue = self.command_queue.lock()
            .map_err(|_| "Failed to acquire command queue lock")?;
        queue.push(command);
        Ok(())
    }

    /// Try to receive a response (non-blocking)
    pub fn try_recv_response(&self) -> Result<Option<WindowResponse>, String> {
        let mut queue = self.response_queue.lock()
            .map_err(|_| "Failed to acquire response queue lock")?;
        Ok(queue.pop())
    }

    /// Run the window manager event loop (consumes self)
    pub fn run(self) -> Result<(), String> {
        let event_loop = EventLoop::new()
            .map_err(|e| format!("Failed to create EventLoop on main thread: {}", e))?;

        let mut window_state: Option<WindowState> = None;
        let command_queue = self.command_queue.clone();
        let response_queue = self.response_queue.clone();

        log::info!("Starting WindowManager event loop on main thread");

        event_loop.run(move |event, event_loop_window_target| {
            // Process commands from the queue
            while let Ok(mut queue) = command_queue.try_lock() {
                if let Some(command) = queue.pop() {
                    log::debug!("Processing command: {:?}", command);

                    match command {
                        WindowCommand::CreateWindow { config, config_dir } => {
                            match Self::create_window_state(config, config_dir, event_loop_window_target) {
                                Ok(state) => {
                                    window_state = Some(state);
                                    Self::send_response(&response_queue, WindowResponse::WindowCreated);
                                }
                                Err(e) => {
                                    Self::send_response(&response_queue, WindowResponse::Error(e));
                                }
                            }
                        }
                        WindowCommand::DestroyWindow => {
                            if window_state.take().is_some() {
                                Self::send_response(&response_queue, WindowResponse::WindowDestroyed);
                                event_loop_window_target.exit();
                                return;
                            }
                        }
                        WindowCommand::UpdateMouse { x, y } => {
                            if let Some(ref mut state) = window_state {
                                state.renderer.handle_mouse_input(x, y);
                                Self::send_response(&response_queue, WindowResponse::MouseUpdated);
                            }
                        }
                    }
                }
                break; // Only process one command per frame
            }

            // Handle winit events
            match event {
                Event::WindowEvent {
                    ref event,
                    window_id: event_window_id,
                } if window_state.as_ref().map(|s| s.window_id) == Some(event_window_id) => {
                    if let Some(ref mut state) = window_state {
                        Self::handle_window_event(event, state, &response_queue, event_loop_window_target);
                    }
                }
                Event::AboutToWait => {
                    // Request redraw for smooth animation
                    if let Some(ref state) = window_state {
                        state.window.request_redraw();
                    }
                }
                _ => {}
            }
        }).map_err(|e| format!("EventLoop error: {}", e))
    }

    fn create_window_state(
        config: Config,
        config_dir: PathBuf,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>
    ) -> Result<WindowState, String> {
        log::info!("Creating window state");

        // Create window
        let window = Rc::new(
            WindowBuilder::new()
                .with_title("shekere Preview")
                .with_inner_size(LogicalSize::new(config.window.width, config.window.height))
                .build(&event_loop)
                .map_err(|e| format!("Failed to create window: {}", e))?
        );

        let window_id = window.id();

        // Create WebGPU surface and context
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(window.as_ref()).unwrap())
                .map_err(|e| format!("Failed to create surface: {}", e))?
        };

        // Get adapter
        let adapter = pollster::block_on(async {
            instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }).await
        }).ok_or("Failed to find suitable adapter")?;

        // Get device and queue
        let (device, queue) = pollster::block_on(async {
            adapter.request_device(
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
            ).await
        }).map_err(|e| format!("Failed to create device: {}", e))?;

        let context = WebGpuContext { device, queue };

        // Configure surface
        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
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

        // Create renderer
        // Leak the config and config_dir to get 'static lifetime
        // This is acceptable for a GUI application where we typically only have one window
        let config_static: &'static Config = Box::leak(Box::new(config));
        let config_dir_static: &'static Path = Box::leak(Box::new(config_dir)).as_path();

        let renderer = pollster::block_on(async move {
            Renderer::new(context, config_static, config_dir_static, size.width, size.height).await
        }).map_err(|e| format!("Failed to create renderer: {}", e))?;

        log::info!("Window state created successfully");

        Ok(WindowState {
            window,
            window_id,
            surface,
            surface_config,
            renderer,
        })
    }

    fn handle_window_event(
        event: &WindowEvent,
        state: &mut WindowState,
        response_queue: &Arc<Mutex<Vec<WindowResponse>>>,
        event_loop: &winit::event_loop::EventLoopWindowTarget<()>,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state: ElementState::Pressed,
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
                ..
            } => {
                log::info!("Window close requested");
                Self::send_response(response_queue, WindowResponse::WindowDestroyed);
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if physical_size.width > 0 && physical_size.height > 0 {
                    state.surface_config.width = physical_size.width;
                    state.surface_config.height = physical_size.height;
                    state.surface.configure(state.renderer.get_device(), &state.surface_config);
                    state.renderer.update_size(physical_size.width, physical_size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                state.renderer.handle_mouse_input(position.x, position.y);
            }
            WindowEvent::RedrawRequested => {
                state.renderer.update(0.0); // delta_time not used currently
                match state.renderer.render_to_surface(&state.surface, &state.surface_config) {
                    Ok(_) => {}
                    Err(shekere_core::RendererError::Surface(
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                    )) => {
                        let size = state.window.inner_size();
                        if size.width > 0 && size.height > 0 {
                            state.surface_config.width = size.width;
                            state.surface_config.height = size.height;
                            state.surface.configure(state.renderer.get_device(), &state.surface_config);
                            state.renderer.update_size(size.width, size.height);
                        }
                    }
                    Err(shekere_core::RendererError::Surface(wgpu::SurfaceError::OutOfMemory)) => {
                        log::error!("OutOfMemory");
                        Self::send_response(response_queue, WindowResponse::Error("OutOfMemory".to_string()));
                        event_loop.exit();
                    }
                    Err(shekere_core::RendererError::Surface(wgpu::SurfaceError::Timeout)) => {
                        log::warn!("Surface timeout");
                    }
                    Err(e) => {
                        log::error!("Render error: {}", e);
                        Self::send_response(response_queue, WindowResponse::Error(format!("Render error: {}", e)));
                    }
                }
            }
            _ => {}
        }
    }

    fn send_response(queue: &Arc<Mutex<Vec<WindowResponse>>>, response: WindowResponse) {
        if let Ok(mut queue) = queue.try_lock() {
            queue.push(response);
        }
    }
}

struct WindowState {
    window: Rc<winit::window::Window>,
    window_id: WindowId,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: Renderer<'static>, // Use 'static lifetime by leaking memory
}

/// Initialize global communication system - called from main()
pub fn init_global_communication() {
    let command_queue = Arc::new(Mutex::new(Vec::new()));
    let response_queue = Arc::new(Mutex::new(Vec::new()));

    // Store the queues globally for Tauri command access
    GLOBAL_COMMAND_QUEUE.set(command_queue).expect("Failed to set global command queue");
    GLOBAL_RESPONSE_QUEUE.set(response_queue).expect("Failed to set global response queue");
}

/// Start a window manager thread when needed
pub fn start_window_manager_thread() -> Result<std::thread::JoinHandle<()>, String> {
    let command_queue = GLOBAL_COMMAND_QUEUE.get()
        .ok_or("Global command queue not initialized")?
        .clone();
    let response_queue = GLOBAL_RESPONSE_QUEUE.get()
        .ok_or("Global response queue not initialized")?
        .clone();

    let handle = std::thread::spawn(move || {
        log::info!("Starting window manager thread");
        let window_manager = WindowManager { command_queue, response_queue };
        if let Err(e) = window_manager.run() {
            log::error!("Window manager error: {}", e);
        }
        log::info!("Window manager thread finished");
    });

    Ok(handle)
}

/// Send a command to the window manager from Tauri commands
pub fn send_command(command: WindowCommand) -> Result<(), String> {
    let queue = GLOBAL_COMMAND_QUEUE.get()
        .ok_or("Global command queue not initialized")?;

    let mut queue_guard = queue.lock().map_err(|_| "Failed to lock command queue")?;
    queue_guard.push(command);
    Ok(())
}

/// Try to receive a response from the window manager
pub fn try_recv_response() -> Result<Option<WindowResponse>, String> {
    let queue = GLOBAL_RESPONSE_QUEUE.get()
        .ok_or("Global response queue not initialized")?;

    let mut queue_guard = queue.lock().map_err(|_| "Failed to lock response queue")?;
    Ok(queue_guard.pop())
}

/// Wait for a response from the window manager (with timeout)
pub fn recv_response_timeout(timeout_ms: u64) -> Result<WindowResponse, String> {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    loop {
        if let Some(response) = try_recv_response()? {
            return Ok(response);
        }

        if start.elapsed() > timeout {
            return Err("Timeout waiting for response".to_string());
        }

        // Sleep briefly to avoid busy waiting
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}