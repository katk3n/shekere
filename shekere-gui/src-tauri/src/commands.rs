use crate::file_tree::{get_file_tree, FileTree};
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
                        extension,
                        index
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
                    pipeline.shader_type,
                    index
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
    // Only store thread management, not the renderer itself
    render_thread: Option<std::thread::JoinHandle<()>>,
    stop_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    // Shared performance metrics
    performance_metrics: std::sync::Arc<std::sync::Mutex<PerformanceMetrics>>,
    // Shared frame data
    frame_data: std::sync::Arc<std::sync::Mutex<Option<SharedFrameData>>>,
}

// Shared performance metrics and frame data between render thread and status queries
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    fps: f32,
    render_time_ms: f32,
    frame_count: u64,
    last_update: std::time::Instant,
}

// Shared frame data for GUI access
#[derive(Debug)]
struct SharedFrameData {
    data: Vec<u8>,
    width: u32,
    height: u32,
    updated: std::time::Instant,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            fps: 0.0,
            render_time_ms: 0.0,
            frame_count: 0,
            last_update: std::time::Instant::now(),
        }
    }
}

// Thread-safe wrapper that communicates with shekere-core::State in separate thread
struct HeadlessRenderer {
    // Communication channels with the shekere-core thread
    frame_receiver: std::sync::mpsc::Receiver<Vec<u8>>,
    stop_sender: std::sync::mpsc::Sender<()>,
    state_thread: Option<std::thread::JoinHandle<()>>,
    fps_tracker: FpsTracker,
    performance_metrics: std::sync::Arc<std::sync::Mutex<PerformanceMetrics>>,
    width: u32,
    height: u32,
}

// FPS tracking for performance metrics
#[derive(Debug)]
struct FpsTracker {
    frame_count: u64,
    last_time: std::time::Instant,
    current_fps: f32,
    frame_times: std::collections::VecDeque<f32>,
}

impl FpsTracker {
    fn new() -> Self {
        Self {
            frame_count: 0,
            last_time: std::time::Instant::now(),
            current_fps: 0.0,
            frame_times: std::collections::VecDeque::with_capacity(60),
        }
    }

    fn update(&mut self) -> f32 {
        let now = std::time::Instant::now();
        let frame_time = now.duration_since(self.last_time).as_millis() as f32;

        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }

        self.frame_count += 1;
        self.last_time = now;

        // Calculate FPS from average frame time
        if !self.frame_times.is_empty() {
            let avg_frame_time: f32 =
                self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
            self.current_fps = if avg_frame_time > 0.0 {
                1000.0 / avg_frame_time
            } else {
                0.0
            };
        }

        frame_time
    }

    fn get_fps(&self) -> f32 {
        self.current_fps
    }
}
impl std::fmt::Debug for HeadlessRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeadlessRenderer")
            .field("fps_tracker", &self.fps_tracker)
            .finish_non_exhaustive()
    }
}
impl HeadlessRenderer {
    pub async fn new(
        config: &shekere_core::Config,
        config_dir: &std::path::Path,
        performance_metrics: std::sync::Arc<std::sync::Mutex<PerformanceMetrics>>,
    ) -> Result<Self, CommandError> {
        let width = config.window.width;
        let height = config.window.height;

        // Create communication channels
        let (frame_sender, frame_receiver) = std::sync::mpsc::channel::<Vec<u8>>();
        let (stop_sender, stop_receiver) = std::sync::mpsc::channel::<()>();

        // Clone data for the thread
        let config_clone = config.clone();
        let config_dir_clone = config_dir.to_path_buf();
        let metrics_clone = performance_metrics.clone();

        // Start shekere-core thread with EventLoop (this will work because it's main thread of this thread)
        let state_thread = std::thread::spawn(move || {
            // This thread will have its own EventLoop, avoiding the macOS main thread issue
            Self::run_shekere_core_loop(
                config_clone,
                config_dir_clone,
                frame_sender,
                stop_receiver,
                metrics_clone,
            );
        });

        Ok(Self {
            frame_receiver,
            stop_sender,
            state_thread: Some(state_thread),
            fps_tracker: FpsTracker::new(),
            performance_metrics,
            width,
            height,
        })
    }

    fn run_shekere_core_loop(
        config: shekere_core::Config,
        config_dir: std::path::PathBuf,
        frame_sender: std::sync::mpsc::Sender<Vec<u8>>,
        stop_receiver: std::sync::mpsc::Receiver<()>,
        performance_metrics: std::sync::Arc<std::sync::Mutex<PerformanceMetrics>>,
    ) {
        // Create headless WebGPU instance without EventLoop/Window to avoid macOS threading issues
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("Failed to create tokio runtime: {}", e);
                return;
            }
        };

        // Initialize headless WebGPU rendering
        let (device, queue, texture, format) =
            match rt.block_on(Self::create_headless_webgpu(&config)) {
                Ok(gpu_resources) => gpu_resources,
                Err(e) => {
                    log::error!("Failed to create headless WebGPU: {}", e);
                    return;
                }
            };

        // Load shaders from config
        let shader_source = match Self::load_shader_from_config(&config, &config_dir) {
            Ok(shader) => shader,
            Err(e) => {
                log::error!("Failed to load shader: {}", e);
                return;
            }
        };

        // Create render pipeline
        let render_pipeline = match Self::create_render_pipeline(&device, &shader_source, format) {
            Ok(pipeline) => pipeline,
            Err(e) => {
                log::error!("Failed to create render pipeline: {}", e);
                return;
            }
        };

        // Create uniform buffers for shekere-core compatibility
        let window_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("WindowBuffer"),
            size: 8, // WindowUniform: vec2<f32> resolution
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let time_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TimeBuffer"),
            size: 4, // TimeUniform: f32 duration
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UniformBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UniformBindGroup"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: window_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: time_buffer.as_entire_binding(),
                },
            ],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let start_time = std::time::Instant::now();
        let mut frame_count = 0u64;

        // Initialize window uniforms
        let window_data = [
            config.window.width as f32,  // resolution.x
            config.window.height as f32, // resolution.y
        ];
        queue.write_buffer(&window_buffer, 0, bytemuck::cast_slice(&window_data));

        // Main render loop
        loop {
            // Check for stop signal
            if stop_receiver.try_recv().is_ok() {
                log::info!("Stopping headless render loop");
                break;
            }

            let elapsed = start_time.elapsed();

            // Update time uniforms (Group 0, Binding 1)
            let time_data = [elapsed.as_secs_f32()]; // duration
            queue.write_buffer(&time_buffer, 0, bytemuck::cast_slice(&time_data));

            // Render frame
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("HeadlessRenderEncoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("HeadlessRenderPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                render_pass.set_pipeline(&render_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.draw(0..4, 0..1); // Fullscreen quad
            }

            queue.submit(std::iter::once(encoder.finish()));

            // Extract frame data
            if let Ok(frame_data) = rt.block_on(Self::extract_frame_data(&device, &queue, &texture))
            {
                if frame_sender.send(frame_data).is_err() {
                    log::warn!("Failed to send frame data - receiver dropped");
                    break;
                }
            }

            // Update performance metrics
            if let Ok(mut metrics) = performance_metrics.lock() {
                metrics.frame_count = frame_count;
                metrics.render_time_ms = 16.7; // Approximate for now
                metrics.fps = if elapsed.as_secs_f32() > 0.0 {
                    frame_count as f32 / elapsed.as_secs_f32()
                } else {
                    0.0
                };
            }

            frame_count += 1;

            // Frame rate limiting (60 FPS target)
            std::thread::sleep(std::time::Duration::from_millis(16));
        }

        log::info!("Headless render loop finished after {} frames", frame_count);
    }

    pub fn render_frame(
        &mut self,
        frame_data_share: Option<std::sync::Arc<std::sync::Mutex<Option<SharedFrameData>>>>,
    ) -> Result<f32, CommandError> {
        // Try to receive latest frame data from shekere-core thread
        if let Ok(frame_data) = self.frame_receiver.try_recv() {
            if let Some(frame_data_arc) = frame_data_share {
                if let Ok(mut shared_data) = frame_data_arc.lock() {
                    *shared_data = Some(SharedFrameData {
                        data: frame_data,
                        width: self.width,
                        height: self.height,
                        updated: std::time::Instant::now(),
                    });
                }
            }
        }

        // Update FPS tracking
        let frame_time = self.fps_tracker.update();

        // Update shared performance metrics
        if let Ok(mut metrics) = self.performance_metrics.lock() {
            metrics.fps = self.fps_tracker.get_fps();
            metrics.render_time_ms = frame_time;
            metrics.frame_count = self.fps_tracker.frame_count;
            metrics.last_update = std::time::Instant::now();
        }

        Ok(frame_time)
    }

    pub fn get_fps(&self) -> f32 {
        self.fps_tracker.get_fps()
    }

    // Helper methods for headless WebGPU rendering
    async fn create_headless_webgpu(
        config: &shekere_core::Config,
    ) -> Result<
        (
            wgpu::Device,
            wgpu::Queue,
            wgpu::Texture,
            wgpu::TextureFormat,
        ),
        CommandError,
    > {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| CommandError::Preview("Failed to find WebGPU adapter".to_string()))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("HeadlessRenderer"),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| CommandError::Preview(format!("Failed to create WebGPU device: {}", e)))?;

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HeadlessRenderTexture"),
            size: wgpu::Extent3d {
                width: config.window.width,
                height: config.window.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        Ok((device, queue, texture, format))
    }

    fn load_shader_from_config(
        config: &shekere_core::Config,
        config_dir: &std::path::Path,
    ) -> Result<String, CommandError> {
        if config.pipeline.is_empty() {
            return Err(CommandError::Preview(
                "No pipeline configuration found".to_string(),
            ));
        }

        // Use the first pipeline entry for now
        let pipeline = &config.pipeline[0];
        let shader_path = config_dir.join(&pipeline.file);

        let shader_content = std::fs::read_to_string(&shader_path).map_err(|e| {
            CommandError::Preview(format!(
                "Failed to read shader file '{}': {}",
                shader_path.display(),
                e
            ))
        })?;

        // Load shekere-core common definitions
        let common_wgsl_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("shaders/common.wgsl");

        let common_content = std::fs::read_to_string(&common_wgsl_path).map_err(|e| {
            CommandError::Preview(format!(
                "Failed to read shekere-core common shader definitions: {}",
                e
            ))
        })?;

        // Create vertex shader for fullscreen quad
        let vertex_shader = r#"
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate fullscreen triangle strip vertices
    // Triangle strip: (-1,-1), (1,-1), (-1,1), (1,1)
    let x = f32((vertex_index & 1u) * 2u) - 1.0;
    let y = f32((vertex_index & 2u)) - 1.0;

    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}
"#;

        // Combine all shader parts
        let complete_shader = format!("{}\n{}\n{}", common_content, vertex_shader, shader_content);

        Ok(complete_shader)
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        shader_source: &str,
        format: wgpu::TextureFormat,
    ) -> Result<wgpu::RenderPipeline, CommandError> {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HeadlessShader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UniformBindGroupLayout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("HeadlessRenderPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("HeadlessRenderPipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(pipeline)
    }

    async fn extract_frame_data(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &wgpu::Texture,
    ) -> Result<Vec<u8>, CommandError> {
        let texture_size = wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        };

        let bytes_per_pixel = 4; // RGBA8UnormSrgb = 4 bytes per pixel
        let unpadded_bytes_per_row = texture_size.width * bytes_per_pixel;

        // WebGPU requires bytes_per_row to be aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256)
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) / align * align;

        let buffer_size = (padded_bytes_per_row * texture_size.height) as u64;

        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FrameDataBuffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Copy texture to buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("FrameDataEncoder"),
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(texture_size.height),
                },
            },
            texture_size,
        );

        queue.submit(std::iter::once(encoder.finish()));

        // Map buffer and read data
        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = tokio::sync::oneshot::channel();

        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        // Poll device until mapping is complete
        device.poll(wgpu::Maintain::Wait);

        // Wait for mapping to complete
        receiver
            .await
            .map_err(|_| {
                CommandError::Preview("Failed to receive buffer mapping result".to_string())
            })?
            .map_err(|e| CommandError::Preview(format!("Failed to map buffer: {:?}", e)))?;

        let mapped_data = buffer_slice.get_mapped_range();

        // Extract the actual image data, removing padding if necessary
        let mut frame_data =
            Vec::with_capacity((unpadded_bytes_per_row * texture_size.height) as usize);

        if padded_bytes_per_row == unpadded_bytes_per_row {
            // No padding, can copy directly
            frame_data.extend_from_slice(&mapped_data);
        } else {
            // Need to remove padding from each row
            for row in 0..texture_size.height {
                let row_start = (row * padded_bytes_per_row) as usize;
                let row_end = row_start + unpadded_bytes_per_row as usize;
                frame_data.extend_from_slice(&mapped_data[row_start..row_end]);
            }
        }

        // Unmap the buffer
        drop(mapped_data);
        output_buffer.unmap();

        Ok(frame_data)
    }
}

impl Drop for HeadlessRenderer {
    fn drop(&mut self) {
        // Send stop signal
        let _ = self.stop_sender.send(());

        // Wait for thread to finish
        if let Some(thread) = self.state_thread.take() {
            let _ = thread.join();
        }
    }
}

// Global preview state management
type PreviewState = Mutex<HashMap<String, PreviewInstance>>;

fn get_preview_state() -> &'static PreviewState {
    static PREVIEW_STATE: std::sync::OnceLock<PreviewState> = std::sync::OnceLock::new();
    PREVIEW_STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[command]
pub async fn start_preview(
    config: shekere_core::Config,
    config_path: Option<String>,
) -> CommandResult<PreviewHandle> {
    // Generate unique preview ID
    let uuid_str = uuid::Uuid::new_v4().to_string();
    let preview_id = format!("preview_{}", &uuid_str[..8]);

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

    // Create shared performance metrics
    let performance_metrics =
        std::sync::Arc::new(std::sync::Mutex::new(PerformanceMetrics::default()));
    let metrics_for_thread = performance_metrics.clone();

    // Create shared frame data
    let frame_data = std::sync::Arc::new(std::sync::Mutex::new(None::<SharedFrameData>));
    let frame_data_for_thread = frame_data.clone();

    // Create stop signal for render thread
    let stop_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_signal_clone = stop_signal.clone();

    // Clone data for the thread
    let config_clone = config.clone();
    let config_dir_clone = config_dir.clone();

    // Start rendering thread - create renderer inside the thread to avoid Send issues
    let render_thread = std::thread::spawn(move || {
        // Create runtime for async operations inside the thread
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("Failed to create tokio runtime: {}", e);
                return;
            }
        };

        // Create the headless renderer inside the thread
        let mut renderer = match rt.block_on(HeadlessRenderer::new(
            &config_clone,
            &config_dir_clone,
            metrics_for_thread,
        )) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to create renderer: {}", e);
                return;
            }
        };

        let mut frame_count = 0u64;

        while !stop_signal_clone.load(std::sync::atomic::Ordering::Relaxed) {
            match renderer.render_frame(Some(frame_data_for_thread.clone())) {
                Ok(_frame_time) => {
                    frame_count += 1;
                    // Simple frame rate limiting (60 FPS target)
                    std::thread::sleep(std::time::Duration::from_millis(16));
                }
                Err(e) => {
                    log::error!("Render error: {}", e);
                    break;
                }
            }
        }

        log::info!("Render thread stopped after {} frames", frame_count);
    });

    // Create preview instance without the renderer
    let instance = PreviewInstance {
        id: preview_id.clone(),
        config: config.clone(),
        config_path: config_path.clone().unwrap_or_else(|| "unknown".to_string()),
        render_thread: Some(render_thread),
        stop_signal: Some(stop_signal),
        performance_metrics,
        frame_data,
    };

    // Store in global state
    {
        let mut state = get_preview_state().lock().map_err(|_| {
            CommandError::Preview("Failed to acquire preview state lock".to_string())
        })?;

        // Stop any existing preview first
        for (_, existing_instance) in state.drain() {
            if let Some(signal) = existing_instance.stop_signal {
                signal.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(thread) = existing_instance.render_thread {
                let _ = thread.join();
            }
        }

        // Add new preview
        state.insert(preview_id.clone(), instance);
    }

    // Return handle with initial status
    Ok(PreviewHandle {
        id: preview_id,
        status: "running".to_string(),
        config_path,
        fps: 60.0,            // Will be updated with real FPS
        render_time_ms: 16.7, // Will be updated with real render time
    })
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

    // Now stop all instances without holding the lock
    for (preview_id, instance) in instances_to_stop {
        log::info!("Stopping preview: {}", preview_id);

        // Signal render thread to stop
        if let Some(stop_signal) = instance.stop_signal {
            stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        // Wait for render thread to finish (with timeout)
        if let Some(render_thread) = instance.render_thread {
            // Use tokio to avoid blocking the async context
            let join_result = tokio::task::spawn_blocking(move || {
                // Give the thread a reasonable time to clean up
                std::thread::sleep(std::time::Duration::from_millis(100));
                render_thread.join()
            })
            .await;

            match join_result {
                Ok(Ok(())) => log::info!("Render thread stopped cleanly"),
                Ok(Err(_)) => log::warn!("Render thread panicked during shutdown"),
                Err(_) => log::warn!("Failed to wait for render thread"),
            }
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

    // Return the first active preview (we only support one for now)
    if let Some((_, instance)) = state.iter().next() {
        // Get real performance metrics
        let (fps, render_time_ms) = if let Ok(metrics) = instance.performance_metrics.lock() {
            (metrics.fps, metrics.render_time_ms)
        } else {
            (0.0, 0.0) // Fallback if metrics are unavailable
        };

        Ok(Some(PreviewHandle {
            id: instance.id.clone(),
            status: "running".to_string(),
            config_path: Some(instance.config_path.clone()),
            fps,
            render_time_ms,
        }))
    } else {
        Ok(None)
    }
}

#[command]
pub async fn get_frame_data() -> CommandResult<Option<Vec<u8>>> {
    let state = get_preview_state()
        .lock()
        .map_err(|_| CommandError::Preview("Failed to acquire preview state lock".to_string()))?;

    // Get frame data from the first active preview
    if let Some((_, instance)) = state.iter().next() {
        if let Ok(frame_data_guard) = instance.frame_data.lock() {
            if let Some(frame_data) = &*frame_data_guard {
                Ok(Some(frame_data.data.clone()))
            } else {
                Ok(Some(vec![])) // No frame data yet
            }
        } else {
            Ok(Some(vec![])) // Failed to lock frame data
        }
    } else {
        Ok(None) // No active preview
    }
}

#[command]
pub async fn get_canvas_dimensions() -> CommandResult<Option<(u32, u32)>> {
    let state = get_preview_state()
        .lock()
        .map_err(|_| CommandError::Preview("Failed to acquire preview state lock".to_string()))?;

    // Return canvas dimensions from the first active preview
    if let Some((_, instance)) = state.iter().next() {
        Ok(Some((
            instance.config.window.width,
            instance.config.window.height,
        )))
    } else {
        Ok(None)
    }
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
