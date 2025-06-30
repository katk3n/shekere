use crate::bind_group_factory::BindGroupFactory;
use crate::hot_reload::HotReloader;
use crate::shader_preprocessor::ShaderPreprocessor;
use crate::timer::Timer;
use crate::uniforms::midi_uniform::MidiUniform;
use crate::uniforms::mouse_uniform::MouseUniform;
use crate::uniforms::osc_uniform::OscUniform;
use crate::uniforms::spectrum_uniform::SpectrumUniform;
use crate::uniforms::time_uniform::TimeUniform;
use crate::uniforms::window_uniform::WindowUniform;
use crate::vertex::{INDICES, VERTICES};
use crate::Config;

use std::path::{Path, PathBuf};
use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};

pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    timer: Timer,

    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: &'a Window,

    // hot reload
    hot_reloader: Option<HotReloader>,
    shader_path: PathBuf,

    // uniforms
    window_uniform: WindowUniform,
    time_uniform: TimeUniform,
    mouse_uniform: MouseUniform,
    osc_uniform: Option<OscUniform<'a>>,
    spectrum_uniform: Option<SpectrumUniform>,
    midi_uniform: Option<MidiUniform>,
    uniform_bind_group: wgpu::BindGroup,
    device_bind_group: wgpu::BindGroup,
    sound_bind_group: Option<wgpu::BindGroup>,
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &'a Window, config: &'a Config, conf_dir: &Path) -> State<'a> {
        let shader_config = &config.pipeline[0];
        let size = window.inner_size();
        let timer = Timer::new();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

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
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

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

        // Uniforms
        let window_uniform = WindowUniform::new(&device, &window);
        let time_uniform = TimeUniform::new(&device);
        let mouse_uniform = MouseUniform::new(&device);
        let osc_uniform = if let Some(osc_config) = &config.osc {
            Some(OscUniform::new(&device, &osc_config).await)
        } else {
            None
        };
        let spectrum_uniform = if let Some(audio_config) = &config.spectrum {
            Some(SpectrumUniform::new(&device, &audio_config))
        } else {
            None
        };
        let midi_uniform = if let Some(midi_config) = &config.midi {
            Some(MidiUniform::new(&device, &midi_config))
        } else {
            None
        };

        // Create bind group for uniforms (window resolution, time)
        let mut uniform_bind_group_factory = BindGroupFactory::new();
        uniform_bind_group_factory.add_entry(WindowUniform::BINDING_INDEX, &window_uniform.buffer);
        uniform_bind_group_factory.add_entry(TimeUniform::BINDING_INDEX, &time_uniform.buffer);
        let (uniform_bind_group_layout, uniform_bind_group) =
            uniform_bind_group_factory.create(&device, "uniform");
        let (uniform_bind_group_layout, uniform_bind_group) = (
            uniform_bind_group_layout.unwrap(),
            uniform_bind_group.unwrap(),
        );

        // Create bind group for device (Mouse, etc.)
        let mut device_bind_group_factory = BindGroupFactory::new();
        device_bind_group_factory.add_entry(MouseUniform::BINDING_INDEX, &mouse_uniform.buffer);
        let (device_bind_group_layout, device_bind_group) =
            device_bind_group_factory.create(&device, "device");
        let (device_bind_group_layout, device_bind_group) = (
            device_bind_group_layout.unwrap(),
            device_bind_group.unwrap(),
        );

        // Create bind group for sound
        let mut sound_bind_group_factory = BindGroupFactory::new();
        if let Some(ou) = &osc_uniform {
            sound_bind_group_factory.add_entry(OscUniform::BINDING_INDEX, &ou.buffer);
        }
        if let Some(su) = &spectrum_uniform {
            sound_bind_group_factory.add_entry(SpectrumUniform::BINDING_INDEX, &su.buffer);
        }
        if let Some(mu) = &midi_uniform {
            sound_bind_group_factory.add_entry(MidiUniform::BINDING_INDEX, &mu.buffer);
        }
        let (sound_bind_group_layout, sound_bind_group) =
            sound_bind_group_factory.create(&device, "sound");

        let mut bind_group_layouts = vec![&uniform_bind_group_layout, &device_bind_group_layout];
        if let Some(layout) = &sound_bind_group_layout {
            bind_group_layouts.push(&layout);
        }

        let shader_path = conf_dir.join(&shader_config.file);

        // Setup hot reload if enabled
        let hot_reloader = if let Some(hot_reload_config) = &config.hot_reload {
            if hot_reload_config.enabled {
                match HotReloader::new(&shader_path) {
                    Ok(reloader) => {
                        log::info!("Hot reload enabled for shader: {:?}", shader_path);
                        Some(reloader)
                    }
                    Err(e) => {
                        log::warn!("Failed to setup hot reload: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let render_pipeline = crate::pipeline::create_pipeline(
            &device,
            conf_dir,
            &shader_config,
            &surface_config,
            &bind_group_layouts,
        );

        // Initialize vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            window_uniform,
            timer,
            time_uniform,
            uniform_bind_group,
            mouse_uniform,
            device_bind_group,
            osc_uniform,
            spectrum_uniform,
            midi_uniform,
            sound_bind_group,
            hot_reloader,
            shader_path,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> &winit::dpi::PhysicalSize<u32> {
        &self.size
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.window_uniform.update(&self.window);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_uniform.update(position);
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self) {
        // Check for shader hot reload
        if let Some(hot_reloader) = &self.hot_reloader {
            if hot_reloader.check_for_changes() {
                match self.reload_shader() {
                    Ok(_) => log::info!("Shader reloaded successfully"),
                    Err(e) => log::error!("Failed to reload shader: {}", e),
                }
            }
        }

        let time_duration = self.timer.get_duration();
        let time_elapsed = time_duration - self.time_uniform.data.duration;
        self.time_uniform.update(time_duration);
        self.time_uniform.write_buffer(&self.queue);

        self.window_uniform.write_buffer(&self.queue);
        self.mouse_uniform.write_buffer(&self.queue);

        // Update OscUniform
        if let Some(osc_uniform) = self.osc_uniform.as_mut() {
            osc_uniform.update(time_elapsed);
            osc_uniform.write_buffer(&self.queue);
        }

        // Update AudioUniform
        if let Some(spectrum_uniform) = self.spectrum_uniform.as_mut() {
            spectrum_uniform.update();
            spectrum_uniform.write_buffer(&self.queue);
        }

        // Update MidiUniform
        if let Some(midi_uniform) = self.midi_uniform.as_mut() {
            midi_uniform.update();
            midi_uniform.write_buffer(&self.queue);
        }
    }

    fn reload_shader(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Reloading shader: {:?}", self.shader_path);

        // Try to create new pipeline safely
        match self.try_create_new_pipeline() {
            Ok(new_pipeline) => {
                // Only update if successful
                self.render_pipeline = new_pipeline;
                log::info!("Shader reloaded successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("Shader hot reload failed: {}", e);
                log::info!("Fix the shader file and save to retry");
                log::info!("Current shader remains active");
                Err(e.into())
            }
        }
    }

    fn try_create_new_pipeline(&self) -> Result<wgpu::RenderPipeline, String> {
        // Process the updated shader file with embedded definitions
        let conf_dir = self.shader_path.parent()
            .ok_or("Failed to get shader directory")?;
        let preprocessor = ShaderPreprocessor::new(conf_dir);
        let fs_str = preprocessor
            .process_file_with_embedded_defs(&self.shader_path)
            .map_err(|e| format!("Failed to process shader file: {}", e))?;

        // Create new fragment shader module with panic catching
        let fragment_shader = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Hot Reloaded Fragment Shader"),
                    source: wgpu::ShaderSource::Wgsl(fs_str.into()),
                })
        })) {
            Ok(shader) => shader,
            Err(_) => {
                return Err("WGSL compilation error - check shader syntax".to_string());
            }
        };

        // Create vertex shader (same as before)
        let vertex_shader = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Vertex Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/vertex.wgsl").into()),
            });

        // Recreate bind group layouts (same as in new())
        let mut uniform_bind_group_factory = BindGroupFactory::new();
        uniform_bind_group_factory
            .add_entry(WindowUniform::BINDING_INDEX, &self.window_uniform.buffer);
        uniform_bind_group_factory.add_entry(TimeUniform::BINDING_INDEX, &self.time_uniform.buffer);
        let (uniform_bind_group_layout, _) =
            uniform_bind_group_factory.create(&self.device, "uniform");
        let uniform_bind_group_layout =
            uniform_bind_group_layout.ok_or("Failed to create uniform bind group layout")?;

        let mut device_bind_group_factory = BindGroupFactory::new();
        device_bind_group_factory
            .add_entry(MouseUniform::BINDING_INDEX, &self.mouse_uniform.buffer);
        let (device_bind_group_layout, _) =
            device_bind_group_factory.create(&self.device, "device");
        let device_bind_group_layout =
            device_bind_group_layout.ok_or("Failed to create device bind group layout")?;

        let mut sound_bind_group_factory = BindGroupFactory::new();
        if let Some(ou) = &self.osc_uniform {
            sound_bind_group_factory.add_entry(OscUniform::BINDING_INDEX, &ou.buffer);
        }
        if let Some(su) = &self.spectrum_uniform {
            sound_bind_group_factory.add_entry(SpectrumUniform::BINDING_INDEX, &su.buffer);
        }
        if let Some(mu) = &self.midi_uniform {
            sound_bind_group_factory.add_entry(MidiUniform::BINDING_INDEX, &mu.buffer);
        }
        let (sound_bind_group_layout, _) = sound_bind_group_factory.create(&self.device, "sound");

        let mut bind_group_layouts = vec![&uniform_bind_group_layout, &device_bind_group_layout];
        if let Some(layout) = &sound_bind_group_layout {
            bind_group_layouts.push(&layout);
        }

        // Create new pipeline layout with panic catching
        let render_pipeline_layout =
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.device
                    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("Hot Reload Render Pipeline Layout"),
                        bind_group_layouts: &bind_group_layouts,
                        push_constant_ranges: &[],
                    })
            })) {
                Ok(layout) => layout,
                Err(_) => {
                    return Err("Failed to create pipeline layout".to_string());
                }
            };

        // Create new render pipeline with panic catching
        let new_render_pipeline =
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("Hot Reload Render Pipeline"),
                        layout: Some(&render_pipeline_layout),
                        vertex: wgpu::VertexState {
                            module: &vertex_shader,
                            entry_point: "vs_main",
                            buffers: &[crate::vertex::Vertex::desc()],
                            compilation_options: wgpu::PipelineCompilationOptions::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &fragment_shader,
                            entry_point: "fs_main",
                            targets: &[Some(wgpu::ColorTargetState {
                                format: self.surface_config.format,
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
            })) {
                Ok(pipeline) => pipeline,
                Err(_) => {
                    return Err(
                        "Failed to create render pipeline - check shader compatibility".to_string(),
                    );
                }
            };

        Ok(new_render_pipeline)
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_bind_group(1, &self.device_bind_group, &[]);

            if let Some(sound_bind_group) = &self.sound_bind_group {
                render_pass.set_bind_group(2, sound_bind_group, &[]);
            }

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
