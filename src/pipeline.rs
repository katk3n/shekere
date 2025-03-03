use std::path::Path;

use crate::vertex::Vertex;
use crate::ShaderConfig;
use wgpu::{BindGroupLayout, Device, RenderPipeline, SurfaceConfiguration};

pub fn create_pipeline(
    device: &Device,
    conf_dir: &Path,
    shader_config: &ShaderConfig,
    surface_config: &SurfaceConfiguration,
    bind_group_layouts: &[&BindGroupLayout],
) -> RenderPipeline {
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Vertex Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/vertex.wgsl").into()),
    });

    let shader_path = conf_dir.join(&shader_config.file);
    let fs_str = std::fs::read_to_string(shader_path.to_str().unwrap()).unwrap();
    let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&shader_config.label),
        source: wgpu::ShaderSource::Wgsl(fs_str.into()),
    });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: bind_group_layouts,
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
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
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
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

    render_pipeline
}
