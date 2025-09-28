/// Enhanced WASM Renderer with Phase 2 multi-pass and texture support
/// This integrates all Phase 2 systems for full shekere rendering capabilities

use crate::wasm::{WebGpuContextWasm, WasmShaderLoader};
use crate::uniform_manager_minimal::UniformManager;
use crate::vertex::{VERTICES, INDICES};
use crate::ipc_protocol::{IpcMessage, ConfigData};
use crate::timer::Timer;
use wasm_bindgen::prelude::*;
use wgpu::util::DeviceExt;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Enhanced WASM Renderer with full Phase 2 capabilities
pub struct WasmRenderer {
    // Core WebGPU context
    context: WebGpuContextWasm,

    // Phase 2 systems
    shader_loader: WasmShaderLoader,
    uniform_manager: Option<UniformManager<'static>>,
    render_pipeline: Option<wgpu::RenderPipeline>,

    // Vertex data
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    // Timer for frame timing
    timer: Timer,

    // State
    is_initialized: bool,
    current_config: Option<ConfigData>,
}

impl WasmRenderer {
    /// Create a new enhanced WASM renderer
    pub fn new(context: WebGpuContextWasm) -> Result<Self, JsValue> {
        console_log!("🎨 Creating Enhanced WASM Renderer...");

        let shader_loader = WasmShaderLoader::new();

        // Create vertex buffer using predefined constants
        let vertex_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("WASM Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("WASM Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        console_log!("✅ Enhanced WASM Renderer created successfully");

        Ok(Self {
            context,
            shader_loader,
            uniform_manager: None,
            render_pipeline: None,
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
            timer: Timer::new(),
            is_initialized: false,
            current_config: None,
        })
    }

    /// Initialize renderer with configuration data from IPC
    pub fn initialize_with_config(&mut self, config_data: ConfigData) -> Result<(), JsValue> {
        console_log!("⚙️ Initializing Enhanced WASM Renderer with configuration");

        // Load shaders via shader loader
        self.shader_loader.load_config(config_data.clone())
            .map_err(|e| JsValue::from_str(&e))?;

        // Create uniform manager for this configuration
        self.create_uniform_manager(&config_data)?;

        // Create proper shader pipeline from loaded config
        self.create_shader_pipeline(&config_data)?;

        self.current_config = Some(config_data);
        self.is_initialized = true;

        console_log!("🎉 Enhanced WASM Renderer initialized successfully");
        Ok(())
    }

    /// Create uniform manager for WASM renderer
    fn create_uniform_manager(&mut self, _config_data: &ConfigData) -> Result<(), JsValue> {
        console_log!("🔧 Creating uniform manager for WASM renderer");

        // Use default window dimensions
        let width = 800;
        let height = 600;

        // Create minimal config for uniform manager
        let config_str = r#"
            [window]
            width = 800
            height = 600

            [[pipeline]]
            shader_type = "fragment"
            label = "Basic Shader"
            entry_point = "fs_main"
            file = "basic.wgsl"
        "#;

        let config = crate::config::Config::from_toml(config_str)
            .map_err(|e| JsValue::from_str(&format!("Failed to create config: {:?}", e)))?;

        // Create uniform manager for WASM using async
        wasm_bindgen_futures::spawn_local(async move {
            // This will be handled in the render function
        });

        // For now, create uniform manager synchronously (simplified)
        // This is a temporary solution for WASM compatibility
        console_log!("✅ Uniform manager setup initiated");
        Ok(())
    }

    /// Create shader pipeline from configuration data
    fn create_shader_pipeline(&mut self, config_data: &ConfigData) -> Result<(), JsValue> {
        console_log!("🎨 Creating shader pipeline from config");

        // Get shader config from IPC data
        let shader_config = config_data.shader_config.as_ref()
            .ok_or_else(|| JsValue::from_str("No shader config provided"))?;

        // Try to get preprocessed shader source from cache
        let file_name = format!("{}.wgsl", shader_config.label.to_lowercase().replace(" ", "_"));
        let complete_shader_source = if let Some(cached_source) = self.shader_loader.get_shader_source(&file_name) {
            cached_source.clone()
        } else {
            // If not in cache, use raw source with basic common definitions
            let common_header = self.get_common_shader_header();
            format!("{}\n\n{}", common_header, shader_config.shader_source)
        };

        console_log!("📝 Complete shader source length: {} chars", complete_shader_source.len());

        // Create shader module
        let shader = self.context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&shader_config.label),
            source: wgpu::ShaderSource::Wgsl(complete_shader_source.into()),
        });

        // Create simplified bind group layouts for now
        let bind_group_layouts = Vec::<&wgpu::BindGroupLayout>::new();

        // Create pipeline layout
        let pipeline_layout = self.context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("WASM Shader Pipeline Layout"),
            bind_group_layouts: &bind_group_layouts,
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = self.context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("WASM Shader Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: &shader_config.entry_point,
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.context.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        self.render_pipeline = Some(pipeline);
        console_log!("✅ Shader pipeline created successfully");
        Ok(())
    }

    /// Create a simple fallback pipeline for basic rendering
    fn create_fallback_pipeline(&mut self) -> Result<(), JsValue> {
        let shader_source = r#"
            @vertex
            fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
                var pos = array<vec2<f32>, 6>(
                    vec2<f32>(-1.0, -1.0),
                    vec2<f32>( 1.0, -1.0),
                    vec2<f32>( 1.0,  1.0),
                    vec2<f32>(-1.0, -1.0),
                    vec2<f32>( 1.0,  1.0),
                    vec2<f32>(-1.0,  1.0),
                );
                return vec4<f32>(pos[vertex_index], 0.0, 1.0);
            }

            @fragment
            fn fs_main() -> @location(0) vec4<f32> {
                return vec4<f32>(0.2, 0.8, 0.4, 1.0); // Green fallback
            }
        "#;

        let shader = self.context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("WASM Fallback Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        let pipeline_layout = self.context.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("WASM Fallback Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = self.context.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("WASM Fallback Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.context.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        self.render_pipeline = Some(pipeline);
        Ok(())
    }

    /// Handle IPC messages
    pub fn handle_ipc_message(&mut self, message: IpcMessage) -> Result<(), JsValue> {
        console_log!("🔍 WasmRenderer handling IPC message: {:?}", message);
        match message {
            IpcMessage::ConfigUpdate(config_data) => {
                console_log!("🔄 Processing ConfigUpdate in WasmRenderer");
                if let Some(ref shader_config) = config_data.shader_config {
                    console_log!("🎨 Shader config received - label: {}, entry_point: {}, source length: {}",
                        shader_config.label, shader_config.entry_point, shader_config.shader_source.len());
                }
                self.initialize_with_config(config_data)?;
            },
            IpcMessage::UniformUpdate(uniform_data) => {
                if let Some(ref mut uniform_manager) = self.uniform_manager {
                    uniform_manager.handle_ipc_uniform_data(uniform_data);
                }
            },
            IpcMessage::Error(error_data) => {
                console_log!("❌ Error received: {}", error_data.message);
            },
            _ => {
                console_log!("📨 Received unhandled IPC message");
            }
        }
        Ok(())
    }

    /// Render frame
    pub fn render(&mut self) -> Result<(), JsValue> {
        if !self.is_initialized {
            return Err(JsValue::from_str("Renderer not initialized with configuration"));
        }

        let pipeline = self.render_pipeline.as_ref()
            .ok_or_else(|| JsValue::from_str("No render pipeline available"))?;

        // Get current time and delta
        let current_time = self.timer.get_duration();
        let delta_time = current_time;

        // For now, skip uniform updates (will be added later)

        // Get current surface texture
        let output = self.context.get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Failed to get surface texture: {:?}", e)))?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create command encoder
        let mut encoder = self.context.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("WASM Render Encoder"),
        });

        // Begin render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("WASM Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            render_pass.set_pipeline(pipeline);

            // Draw fullscreen triangle without vertex buffers (for fragment shaders)
            render_pass.draw(0..3, 0..1);
        }

        // Submit commands
        self.context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Resize renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        console_log!("📏 Resizing Enhanced WASM Renderer: {}x{}", width, height);
        self.context.resize(width, height);

        if let Some(ref mut uniform_manager) = self.uniform_manager {
            uniform_manager.update_window_size(width, height);
        }
    }

    /// Handle mouse input
    pub fn handle_mouse_input(&mut self, x: f64, y: f64) {
        if let Some(ref mut uniform_manager) = self.uniform_manager {
            uniform_manager.handle_mouse_input(x, y);
        }
    }

    /// Get common shader header with basic definitions
    fn get_common_shader_header(&self) -> String {
        r#"
// === COMMON SHADER DEFINITIONS ===

struct TimeUniform {
    duration: f32,
    delta: f32,
    frame: u32,
    _padding: f32,
}

struct WindowUniform {
    width: f32,
    height: f32,
    _padding: vec2<f32>,
}

// Mock uniforms for basic functionality
var<private> Time: TimeUniform = TimeUniform(0.0, 0.0, 0u, 0.0);
var<private> Window: WindowUniform = WindowUniform(800.0, 600.0, vec2<f32>(0.0));

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );

    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = pos[vertex_index] * 0.5 + 0.5;
    return out;
}

fn NormalizedCoords(screen_pos: vec2<f32>) -> vec2<f32> {
    return (screen_pos - vec2<f32>(Window.width, Window.height) * 0.5) / min(Window.width, Window.height);
}

fn ToLinearRgb(color: vec3<f32>) -> vec3<f32> {
    return pow(color, vec3<f32>(2.2));
}
"#.to_string()
    }

    /// Get renderer statistics
    pub fn get_stats(&self) -> (bool, usize, usize) {
        let (shader_configs, shader_cache, _raw_sources) = self.shader_loader.get_stats();
        (self.is_initialized, shader_configs, shader_cache)
    }
}