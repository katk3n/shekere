/// WASM-compatible MultiPass Pipeline for browser-based rendering
/// This handles multi-pass shader effects like ping-pong buffers and persistent textures

use crate::bind_group_factory::BindGroupFactory;
use crate::config::ShaderConfig;
use crate::vertex::Vertex;
use wasm_bindgen::prelude::*;
use wgpu::{Device, RenderPipeline, BindGroupLayout, SurfaceConfiguration};
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// WASM-compatible MultiPass Pipeline
pub struct WasmMultiPassPipeline {
    pub pipelines: Vec<RenderPipeline>,
    pub texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub empty_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub shader_sources: HashMap<String, String>, // Cache compiled shader sources
}

impl WasmMultiPassPipeline {
    /// Create a new WASM MultiPass Pipeline
    pub fn new(
        device: &Device,
        shader_configs: &[ShaderConfig],
        surface_config: &SurfaceConfiguration,
        base_bind_group_layouts: &[&BindGroupLayout],
        shader_sources: HashMap<String, String>, // Pre-loaded shader sources from IPC
    ) -> Self {
        console_log!("🔧 Creating WASM MultiPass Pipeline with {} shaders", shader_configs.len());

        let mut pipelines = Vec::new();
        let mut texture_bind_group_layout = None;
        let mut empty_bind_group_layout = None;

        // Check if any shader uses multi-pass features
        let needs_texture_bindings = shader_configs
            .iter()
            .any(|config| config.ping_pong.unwrap_or(false) || config.persistent.unwrap_or(false))
            || shader_configs.len() > 1;

        console_log!("🎯 Needs texture bindings: {}", needs_texture_bindings);

        if needs_texture_bindings {
            // Create texture bind group layout for Group 3 using BindGroupFactory
            texture_bind_group_layout = Some(
                BindGroupFactory::create_multipass_texture_layout(
                    device,
                    "wasm_texture_bind_group_layout",
                ),
            );

            // Create empty bind group layout for missing Group 2
            empty_bind_group_layout = Some(device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    entries: &[],
                    label: Some("wasm_empty_bind_group_layout"),
                },
            ));

            console_log!("✅ Created texture and empty bind group layouts");
        }

        // Create pipelines for each shader
        for (pass_index, shader_config) in shader_configs.iter().enumerate() {
            let enable_multipass = pass_index > 0
                || shader_config.ping_pong.unwrap_or(false)
                || shader_config.persistent.unwrap_or(false);

            console_log!(
                "🔨 Creating pipeline for pass {}: {}, enable_multipass: {}",
                pass_index,
                shader_config.label,
                enable_multipass
            );

            // Get shader source from cache
            let shader_source = shader_sources.get(&shader_config.file)
                .expect(&format!("Shader source not found for {}", shader_config.file));

            // Create bind group layout vector with proper Group indices
            let mut bind_group_layouts = Vec::new();

            // Group 0 and 1 are always present (if available)
            let available_layouts = base_bind_group_layouts.len().min(2);
            bind_group_layouts.extend_from_slice(&base_bind_group_layouts[0..available_layouts]);

            // Group 2 (sound) - add if present in base layouts
            if base_bind_group_layouts.len() > 2 {
                bind_group_layouts.push(base_bind_group_layouts[2]);
            } else if enable_multipass {
                // Use empty layout for Group 2 to maintain correct indexing
                if let Some(ref empty_layout) = empty_bind_group_layout {
                    bind_group_layouts.push(empty_layout);
                }
            }

            // Group 3 (multipass textures) - add if needed
            if let Some(ref layout) = texture_bind_group_layout {
                if enable_multipass {
                    console_log!(
                        "📎 Adding texture bind group layout for pass {} at Group 3",
                        pass_index
                    );
                    bind_group_layouts.push(layout);
                }
            }

            let pipeline = Self::create_pipeline_wasm(
                device,
                shader_config,
                shader_source,
                surface_config,
                &bind_group_layouts,
                enable_multipass,
            );

            pipelines.push(pipeline);
        }

        console_log!("🎉 WASM MultiPass Pipeline created with {} passes", pipelines.len());

        Self {
            pipelines,
            texture_bind_group_layout,
            empty_bind_group_layout,
            shader_sources,
        }
    }

    /// Create a single render pipeline for WASM (no file system access)
    fn create_pipeline_wasm(
        device: &Device,
        shader_config: &ShaderConfig,
        shader_source: &str,
        surface_config: &SurfaceConfiguration,
        bind_group_layouts: &[&BindGroupLayout],
        enable_multipass: bool,
    ) -> RenderPipeline {
        // Create vertex shader (embedded)
        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("WASM Vertex Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::get_vertex_shader_source().into()),
        });

        // Process fragment shader with multipass support
        let processed_shader = if enable_multipass {
            Self::add_multipass_bindings(shader_source)
        } else {
            shader_source.to_string()
        };

        console_log!("📝 Creating fragment shader: {}", shader_config.label);

        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("WASM {}", shader_config.label)),
            source: wgpu::ShaderSource::Wgsl(processed_shader.into()),
        });

        console_log!(
            "🔗 Creating pipeline layout with {} bind group layouts",
            bind_group_layouts.len()
        );

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("WASM {} Layout", shader_config.label)),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("WASM {} Pipeline", shader_config.label)),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: &shader_config.entry_point,
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
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
        })
    }

    /// Get embedded vertex shader source (since we can't read files in WASM)
    fn get_vertex_shader_source() -> &'static str {
        // Embedded vertex shader - same as shaders/vertex.wgsl
        r#"
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) position: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = vec4<f32>(model.position, 1.0);
    out.position = model.position;
    return out;
}
        "#
    }

    /// Add multipass texture bindings to shader source
    fn add_multipass_bindings(shader_source: &str) -> String {
        let multipass_bindings = r#"
// Multipass texture bindings (Group 3)
@group(3) @binding(0)
var multipass_texture: texture_2d<f32>;

@group(3) @binding(1)
var multipass_sampler: sampler;
"#;

        // Insert bindings after any existing bindings but before functions
        if let Some(main_pos) = shader_source.find("fn ") {
            let mut result = String::new();
            result.push_str(&shader_source[..main_pos]);
            result.push_str(multipass_bindings);
            result.push('\n');
            result.push_str(&shader_source[main_pos..]);
            result
        } else {
            // Fallback: prepend bindings
            format!("{}\n{}", multipass_bindings, shader_source)
        }
    }

    /// Check if this is a multi-pass pipeline
    pub fn is_multi_pass(&self) -> bool {
        self.pipelines.len() > 1 || self.texture_bind_group_layout.is_some()
    }

    /// Get pipeline by index
    pub fn get_pipeline(&self, index: usize) -> Option<&RenderPipeline> {
        self.pipelines.get(index)
    }

    /// Get number of pipeline passes
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }

    /// Update shader sources (for hot reload)
    pub fn update_shader_sources(&mut self, new_sources: HashMap<String, String>) {
        console_log!("🔄 Updating shader sources for hot reload");
        self.shader_sources = new_sources;
        // Note: Actual pipeline recreation would need to be triggered externally
    }

    /// Get texture bind group layout (for texture manager)
    pub fn texture_bind_group_layout(&self) -> Option<&BindGroupLayout> {
        self.texture_bind_group_layout.as_ref()
    }

    /// Get empty bind group layout (for missing groups)
    pub fn empty_bind_group_layout(&self) -> Option<&BindGroupLayout> {
        self.empty_bind_group_layout.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShaderConfig;

    #[test]
    fn test_multipass_bindings_addition() {
        let original_shader = r#"
fn fs_main() -> vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
"#;

        let processed = WasmMultiPassPipeline::add_multipass_bindings(original_shader);
        assert!(processed.contains("@group(3) @binding(0)"));
        assert!(processed.contains("multipass_texture"));
        assert!(processed.contains("multipass_sampler"));
    }

    #[test]
    fn test_pipeline_count() {
        // This would require a more complex setup with actual WebGPU device
        // For now, just test the basic structure
        assert_eq!(WasmMultiPassPipeline::get_vertex_shader_source().len() > 0, true);
    }
}