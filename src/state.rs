use crate::osc;
use crate::timer::Timer;
use crate::uniforms::mouse_uniform::MouseUniform;
use crate::uniforms::osc_uniform::OscUniform;
use crate::uniforms::time_uniform::TimeUniform;
use crate::uniforms::window_uniform::WindowUniform;
use crate::vertex::{INDICES, VERTICES};
use crate::ShaderConfig;

use async_std::channel::Receiver;
use rosc::OscPacket;
use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};

pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
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

    osc_receiver: Receiver<OscPacket>,

    // uniforms
    window_uniform: WindowUniform,
    time_uniform: TimeUniform,
    mouse_uniform: MouseUniform,
    osc_uniform: OscUniform,
    uniform_bind_group: wgpu::BindGroup,
    device_bind_group: wgpu::BindGroup,
    sound_bind_group: wgpu::BindGroup,
}

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &'a Window, shader_config: &ShaderConfig) -> State<'a> {
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
        let config = wgpu::SurfaceConfiguration {
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

        let osc_receiver = osc::osc_start().await;

        // Uniforms
        let window_uniform = WindowUniform::new(&device, &window);
        let time_uniform = TimeUniform::new(&device);
        let mouse_uniform = MouseUniform::new(&device);
        let osc_uniform = OscUniform::new(&device);

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: WindowUniform::BINDING_INDEX,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: TimeUniform::BINDING_INDEX,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("uniform_bind_group_layout"),
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: WindowUniform::BINDING_INDEX,
                    resource: window_uniform.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: TimeUniform::BINDING_INDEX,
                    resource: time_uniform.buffer.as_entire_binding(),
                },
            ],
            label: Some("uniform_binding_group"),
        });

        let device_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: MouseUniform::BINDING_INDEX,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("device_bind_group_layout"),
            });
        let device_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &device_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: MouseUniform::BINDING_INDEX,
                resource: mouse_uniform.buffer.as_entire_binding(),
            }],
            label: Some("device_binding_group"),
        });

        let sound_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: OscUniform::BINDING_INDEX,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("sound_bind_group_layout"),
            });
        let sound_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &sound_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: OscUniform::BINDING_INDEX,
                resource: osc_uniform.buffer.as_entire_binding(),
            }],
            label: Some("sound_binding_group"),
        });

        let bind_group_layouts = [
            &uniform_bind_group_layout,
            &device_bind_group_layout,
            &sound_bind_group_layout,
        ];

        let render_pipeline =
            crate::pipeline::create_pipeline(&device, &shader_config, &config, &bind_group_layouts);

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
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            osc_receiver,
            window_uniform,
            timer,
            time_uniform,
            uniform_bind_group,
            mouse_uniform,
            device_bind_group,
            osc_uniform,
            sound_bind_group,
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
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
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
        let time_duration = self.timer.get_duration();
        let time_delta = time_duration - self.time_uniform.data.duration;

        // Update TimeUniform
        self.time_uniform.update(time_duration);

        // Update OscUniform
        match self.osc_receiver.try_recv() {
            Ok(packet) => {
                self.osc_uniform.update(packet);
            }
            Err(_) => {
                self.osc_uniform.elapse(time_delta);
            }
        }

        // Write buffer
        self.window_uniform.write_buffer(&self.queue);
        self.time_uniform.write_buffer(&self.queue);
        self.mouse_uniform.write_buffer(&self.queue);
        self.osc_uniform.write_buffer(&self.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
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
            render_pass.set_bind_group(2, &self.sound_bind_group, &[]);
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
