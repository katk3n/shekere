use std::path::Path;

use crate::ShaderConfig;
use crate::shader_preprocessor::ShaderPreprocessor;
use crate::vertex::Vertex;
use wgpu::{BindGroupLayout, Device, RenderPipeline, SurfaceConfiguration};

pub fn create_pipeline(
    device: &Device,
    conf_dir: &Path,
    shader_config: &ShaderConfig,
    surface_config: &SurfaceConfiguration,
    bind_group_layouts: &[&BindGroupLayout],
) -> RenderPipeline {
    create_pipeline_with_multipass(
        device,
        conf_dir,
        shader_config,
        surface_config,
        bind_group_layouts,
        false,
    )
}

pub fn create_pipeline_with_multipass(
    device: &Device,
    conf_dir: &Path,
    shader_config: &ShaderConfig,
    surface_config: &SurfaceConfiguration,
    bind_group_layouts: &[&BindGroupLayout],
    enable_multipass: bool,
) -> RenderPipeline {
    let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Vertex Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/vertex.wgsl").into()),
    });

    let shader_path = conf_dir.join(&shader_config.file);
    let preprocessor = ShaderPreprocessor::new(conf_dir);
    let fs_str = preprocessor
        .process_file_with_embedded_defs_and_multipass(&shader_path, enable_multipass)
        .unwrap();

    let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&shader_config.label),
        source: wgpu::ShaderSource::Wgsl(fs_str.into()),
    });

    log::info!(
        "Creating pipeline layout with {} bind group layouts",
        bind_group_layouts.len()
    );
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts,
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

pub struct MultiPassPipeline {
    pub pipelines: Vec<RenderPipeline>,
    pub texture_bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub empty_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl MultiPassPipeline {
    pub fn new(
        device: &Device,
        conf_dir: &Path,
        shader_configs: &[ShaderConfig],
        surface_config: &SurfaceConfiguration,
        base_bind_group_layouts: &[&BindGroupLayout],
    ) -> Self {
        let mut pipelines = Vec::new();
        let mut texture_bind_group_layout = None;
        let mut empty_bind_group_layout = None;

        // Check if any shader uses multi-pass features
        let needs_texture_bindings = shader_configs
            .iter()
            .any(|config| config.ping_pong.unwrap_or(false) || config.persistent.unwrap_or(false))
            || shader_configs.len() > 1;

        if needs_texture_bindings {
            // Create texture bind group layout for Group 3
            texture_bind_group_layout = Some(device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        // Binding 0: Texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        // Binding 1: Sampler
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                },
            ));

            // Create empty bind group layout for missing Group 2
            empty_bind_group_layout = Some(device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    entries: &[],
                    label: Some("empty_bind_group_layout"),
                },
            ));
        }

        // Create pipelines for each shader
        for (pass_index, shader_config) in shader_configs.iter().enumerate() {
            let enable_multipass = pass_index > 0
                || shader_config.ping_pong.unwrap_or(false)
                || shader_config.persistent.unwrap_or(false);

            log::info!(
                "Creating pipeline for pass {}: {}, enable_multipass: {}",
                pass_index,
                shader_config.label,
                enable_multipass
            );

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
                    log::info!(
                        "Adding texture bind group layout for pass {} at Group 3",
                        pass_index
                    );
                    bind_group_layouts.push(layout);
                }
            }

            let pipeline = create_pipeline_with_multipass(
                device,
                conf_dir,
                shader_config,
                surface_config,
                &bind_group_layouts,
                enable_multipass,
            );

            pipelines.push(pipeline);
        }

        Self {
            pipelines,
            texture_bind_group_layout,
            empty_bind_group_layout,
        }
    }

    pub fn is_multi_pass(&self) -> bool {
        self.pipelines.len() > 1 || self.texture_bind_group_layout.is_some()
    }

    pub fn get_pipeline(&self, index: usize) -> Option<&RenderPipeline> {
        self.pipelines.get(index)
    }

    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShaderConfig;

    fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            });

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();

            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        label: None,
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .unwrap()
        })
    }

    fn create_test_surface_config() -> wgpu::SurfaceConfiguration {
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: 800,
            height: 600,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        }
    }

    #[test]
    fn test_single_pass_pipeline_creation() {
        let (device, _queue) = create_test_device();
        let surface_config = create_test_surface_config();

        let shader_configs = vec![ShaderConfig {
            shader_type: "fragment".to_string(),
            label: "Basic Shader".to_string(),
            entry_point: "fs_main".to_string(),
            file: "tests/shaders/basic.wgsl".to_string(),
            ping_pong: None,
            persistent: None,
        }];

        let multi_pass_pipeline = MultiPassPipeline::new(
            &device,
            Path::new("."),
            &shader_configs,
            &surface_config,
            &[],
        );

        assert_eq!(multi_pass_pipeline.pipeline_count(), 1);
        assert!(!multi_pass_pipeline.is_multi_pass()); // Single pass without special flags
        assert!(multi_pass_pipeline.get_pipeline(0).is_some());
        assert!(multi_pass_pipeline.get_pipeline(1).is_none());
    }

    #[test]
    fn test_multi_pass_pipeline_creation() {
        let (device, _queue) = create_test_device();
        let surface_config = create_test_surface_config();

        let shader_configs = vec![
            ShaderConfig {
                shader_type: "fragment".to_string(),
                label: "Main Render".to_string(),
                entry_point: "fs_main".to_string(),
                file: "tests/shaders/main.wgsl".to_string(),
                ping_pong: None,
                persistent: None,
            },
            ShaderConfig {
                shader_type: "fragment".to_string(),
                label: "Blur Effect".to_string(),
                entry_point: "fs_main".to_string(),
                file: "tests/shaders/blur.wgsl".to_string(),
                ping_pong: None,
                persistent: None,
            },
        ];

        let multi_pass_pipeline = MultiPassPipeline::new(
            &device,
            Path::new("."),
            &shader_configs,
            &surface_config,
            &[],
        );

        assert_eq!(multi_pass_pipeline.pipeline_count(), 2);
        assert!(multi_pass_pipeline.is_multi_pass()); // Multi-pass because > 1 pipeline
        assert!(multi_pass_pipeline.get_pipeline(0).is_some());
        assert!(multi_pass_pipeline.get_pipeline(1).is_some());
        assert!(multi_pass_pipeline.get_pipeline(2).is_none());
        assert!(multi_pass_pipeline.texture_bind_group_layout.is_some());
    }

    #[test]
    fn test_ping_pong_pipeline_creation() {
        let (device, _queue) = create_test_device();
        let surface_config = create_test_surface_config();

        let shader_configs = vec![ShaderConfig {
            shader_type: "fragment".to_string(),
            label: "Game of Life".to_string(),
            entry_point: "fs_main".to_string(),
            file: "tests/shaders/life.wgsl".to_string(),
            ping_pong: Some(true),
            persistent: None,
        }];

        let multi_pass_pipeline = MultiPassPipeline::new(
            &device,
            Path::new("."),
            &shader_configs,
            &surface_config,
            &[],
        );

        assert_eq!(multi_pass_pipeline.pipeline_count(), 1);
        assert!(multi_pass_pipeline.is_multi_pass()); // Multi-pass because ping_pong is enabled
        assert!(multi_pass_pipeline.get_pipeline(0).is_some());
        assert!(multi_pass_pipeline.texture_bind_group_layout.is_some());
        assert_eq!(shader_configs[0].ping_pong, Some(true));
    }
}
