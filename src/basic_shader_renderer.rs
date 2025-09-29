// Basic shader rendering implementation for Bevy
// This implements a simple fullscreen quad renderer with WGSL shaders

use bevy::prelude::*;
use bevy::render::{
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    RenderApp, RenderSet,
    render_graph::{RenderGraph, Node, NodeRunError, RenderGraphContext, RenderLabel},
    Render, ExtractSchedule,
    view::ViewTarget,
};
use std::borrow::Cow;

use crate::vertex::{VERTICES, INDICES, Vertex};
use crate::uniforms::window_uniform::WindowUniform;
use crate::uniforms::time_uniform::TimeUniform;

// Plugin for basic shader rendering
pub struct BasicShaderRenderPlugin;

impl Plugin for BasicShaderRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(ExtractSchedule, extract_shader_data);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(Render, prepare_shader_pipeline.in_set(RenderSet::Prepare))
                .add_systems(Render, queue_shader_render.in_set(RenderSet::Queue));

            let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
            render_graph.add_node(BasicShaderLabel, BasicShaderNode::default());
            render_graph.add_node_edge(
                bevy::render::graph::CameraDriverLabel,
                BasicShaderLabel,
            );
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct BasicShaderLabel;

// Extracted data from main world
#[derive(Resource, Default)]
struct ExtractedShaderData {
    pub config: Option<crate::config::Config>,
    pub config_dir: Option<std::path::PathBuf>,
    pub time: f32,
    pub window_width: f32,
    pub window_height: f32,
}

// Render world resources
#[derive(Resource)]
struct ShaderRenderData {
    pub pipeline: RenderPipeline,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub uniform_bind_group: BindGroup,
    pub uniform_buffer: Buffer,
    pub time_buffer: Buffer,
}

// Extract data from main world to render world
fn extract_shader_data(
    mut commands: Commands,
    config: Res<crate::ShekerConfig>,
    time: Res<Time>,
    windows: Query<&Window>,
) {
    let window = windows.get_single().unwrap_or(&Window::default());

    commands.insert_resource(ExtractedShaderData {
        config: Some(config.config.clone()),
        config_dir: Some(config.config_dir.clone()),
        time: time.elapsed_secs(),
        window_width: window.width(),
        window_height: window.height(),
    });
}

// Prepare shader pipeline and resources
fn prepare_shader_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    extracted_data: Res<ExtractedShaderData>,
    mut render_data: Option<ResMut<ShaderRenderData>>,
) {
    if render_data.is_none() {
        if let (Some(config), Some(config_dir)) = (&extracted_data.config, &extracted_data.config_dir) {
            log::info!("Creating basic shader pipeline");

            // Create vertex buffer
            let vertex_buffer = render_device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: BufferUsages::VERTEX,
            });

            // Create index buffer
            let index_buffer = render_device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: BufferUsages::INDEX,
            });

            // Create uniform buffers
            let uniform_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("Window Uniform Buffer"),
                size: std::mem::size_of::<WindowUniform>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let time_buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("Time Uniform Buffer"),
                size: std::mem::size_of::<TimeUniform>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Create bind group layout
            let bind_group_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

            // Create bind group
            let uniform_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("Uniform Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: time_buffer.as_entire_binding(),
                    },
                ],
            });

            // Load and combine shaders
            let vertex_shader = include_str!("../shaders/vertex.wgsl");
            let common_shader = include_str!("../shaders/common.wgsl");

            // Load fragment shader from config
            let fragment_path = config_dir.join(&config.pipeline[0].file);
            let fragment_shader = match std::fs::read_to_string(&fragment_path) {
                Ok(content) => content,
                Err(e) => {
                    log::error!("Failed to load fragment shader {:?}: {}", fragment_path, e);
                    // Use a simple fallback shader
                    String::from(r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0); // Magenta fallback
}
"#)
                }
            };

            // Combine shaders: common + vertex + fragment
            let combined_shader = format!("{}\n{}\n{}", common_shader, vertex_shader, fragment_shader);

            // Create shader module
            let shader_module = render_device.create_shader_module(ShaderModuleDescriptor {
                label: Some("Basic Shader"),
                source: ShaderSource::Wgsl(Cow::Owned(combined_shader)),
            });

            // Create pipeline layout
            let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Basic Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            // Create render pipeline
            let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Basic Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: VertexStepMode::Vertex,
                        attributes: &[VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: VertexFormat::Float32x3,
                        }],
                    }],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(ColorTargetState {
                        format: TextureFormat::Bgra8UnormSrgb,
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            });

            commands.insert_resource(ShaderRenderData {
                pipeline,
                vertex_buffer,
                index_buffer,
                uniform_bind_group,
                uniform_buffer,
                time_buffer,
            });

            log::info!("Basic shader pipeline created successfully");
        }
    }

    // Update uniform buffers every frame
    if let Some(render_data) = render_data.as_deref() {
        // Update window uniform
        let window_uniform = WindowUniform {
            resolution: [extracted_data.window_width, extracted_data.window_height],
        };
        render_queue.write_buffer(
            &render_data.uniform_buffer,
            0,
            bytemuck::cast_slice(&[window_uniform]),
        );

        // Update time uniform
        let time_uniform = TimeUniform {
            duration: extracted_data.time,
        };
        render_queue.write_buffer(
            &render_data.time_buffer,
            0,
            bytemuck::cast_slice(&[time_uniform]),
        );
    }
}

// Queue render commands
fn queue_shader_render(
    render_data: Option<Res<ShaderRenderData>>,
) {
    if let Some(_render_data) = render_data {
        // Render commands will be issued in the render node
    }
}

// Render graph node
#[derive(Default)]
struct BasicShaderNode;

impl Node for BasicShaderNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let render_data = world.resource::<ShaderRenderData>();

        // Get the main window surface
        let view_targets = world.resource::<ViewTarget>();

        let mut render_pass = render_context.command_encoder().begin_render_pass(&RenderPassDescriptor {
            label: Some("Basic Shader Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: view_targets.main_texture_view(),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::rgba(0.0, 0.0, 0.0, 1.0).into()),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&render_data.pipeline);
        render_pass.set_bind_group(0, &render_data.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, render_data.vertex_buffer.slice(..));
        render_pass.set_index_buffer(render_data.index_buffer.slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);

        drop(render_pass);

        Ok(())
    }
}