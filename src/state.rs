use crate::Config;
use crate::bind_group_factory::BindGroupFactory;
use crate::hot_reload::HotReloader;
use crate::pipeline::MultiPassPipeline;
// use crate::shader_preprocessor::ShaderPreprocessor; // TODO: Needed for hot reload
use crate::texture_manager::{TextureManager, TextureType};
use crate::timer::Timer;
use crate::uniforms::midi_uniform::MidiUniform;
use crate::uniforms::mouse_uniform::MouseUniform;
use crate::uniforms::osc_uniform::OscUniform;
use crate::uniforms::spectrum_uniform::SpectrumUniform;
use crate::uniforms::time_uniform::TimeUniform;
use crate::uniforms::window_uniform::WindowUniform;
use crate::vertex::{INDICES, VERTICES};

use std::path::Path;
use wgpu::util::DeviceExt;
use winit::{event::*, window::Window};

pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    // Multi-pass pipeline support
    multi_pass_pipeline: MultiPassPipeline,
    texture_manager: TextureManager,

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
    config: Config,
    config_dir: std::path::PathBuf,

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
    pub async fn new(
        window: &'a Window,
        config: &'a Config,
        conf_dir: &Path,
    ) -> Result<State<'a>, Box<dyn std::error::Error>> {
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

        // Setup hot reload if enabled - watch all shader files in the pipeline
        let hot_reloader = if let Some(hot_reload_config) = &config.hot_reload {
            if hot_reload_config.enabled {
                let shader_paths: Vec<std::path::PathBuf> = config
                    .pipeline
                    .iter()
                    .map(|shader_config| conf_dir.join(&shader_config.file))
                    .collect();

                match HotReloader::new_multi_file(shader_paths.clone()) {
                    Ok(reloader) => {
                        log::info!(
                            "Hot reload enabled for {} shader files: {:?}",
                            shader_paths.len(),
                            shader_paths
                        );
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

        // Old single-pass pipeline code removed - now using MultiPassPipeline

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

        // Initialize multi-pass pipeline and texture manager
        let multi_pass_pipeline = MultiPassPipeline::new(
            &device,
            conf_dir,
            &config.pipeline,
            &surface_config,
            &bind_group_layouts,
        );

        let texture_manager = TextureManager::new_with_format(
            &device,
            size.width,
            size.height,
            surface_config.format.add_srgb_suffix(),
        );

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            size,
            multi_pass_pipeline,
            texture_manager,
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
            config: config.clone(),
            config_dir: conf_dir.to_path_buf(),
        })
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
            // Clear textures on resize
            self.texture_manager.clear_all_textures();
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
        log::info!("Attempting to reload multi-pass shaders...");

        match self.try_create_new_multi_pass_pipeline(&self.config_dir) {
            Ok(new_pipeline) => {
                // Replace the old pipeline with the new one
                self.multi_pass_pipeline = new_pipeline;

                // Clear texture manager state to avoid potential issues with stale textures
                self.texture_manager.clear_all_textures();

                log::info!("Multi-pass shaders reloaded successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to recreate multi-pass pipeline: {}", e);
                // Keep the old pipeline working
                Err(format!("Shader compilation failed: {}", e).into())
            }
        }
    }

    fn try_create_new_multi_pass_pipeline(
        &self,
        conf_dir: &std::path::Path,
    ) -> Result<crate::pipeline::MultiPassPipeline, String> {
        use std::panic::{self, AssertUnwindSafe};

        // Use existing bind group layouts to ensure compatibility
        // We need to recreate the bind group layouts using the same method as in new()

        // Recreate uniform bind group layout using BindGroupFactory (same as original)
        let mut uniform_bind_group_factory = crate::bind_group_factory::BindGroupFactory::new();
        uniform_bind_group_factory.add_entry(
            crate::uniforms::window_uniform::WindowUniform::BINDING_INDEX,
            &self.window_uniform.buffer,
        );
        uniform_bind_group_factory.add_entry(
            crate::uniforms::time_uniform::TimeUniform::BINDING_INDEX,
            &self.time_uniform.buffer,
        );
        let (uniform_bind_group_layout, _) =
            uniform_bind_group_factory.create(&self.device, "uniform");
        let uniform_bind_group_layout = uniform_bind_group_layout.unwrap();

        // Recreate device bind group layout using BindGroupFactory (same as original)
        let mut device_bind_group_factory = crate::bind_group_factory::BindGroupFactory::new();
        device_bind_group_factory.add_entry(
            crate::uniforms::mouse_uniform::MouseUniform::BINDING_INDEX,
            &self.mouse_uniform.buffer,
        );
        let (device_bind_group_layout, _) =
            device_bind_group_factory.create(&self.device, "device");
        let device_bind_group_layout = device_bind_group_layout.unwrap();

        // Recreate sound bind group layout using BindGroupFactory (same as original)
        let mut sound_bind_group_factory = crate::bind_group_factory::BindGroupFactory::new();
        if let Some(ou) = &self.osc_uniform {
            sound_bind_group_factory.add_entry(
                crate::uniforms::osc_uniform::OscUniform::BINDING_INDEX,
                &ou.buffer,
            );
        }
        if let Some(su) = &self.spectrum_uniform {
            sound_bind_group_factory.add_entry(
                crate::uniforms::spectrum_uniform::SpectrumUniform::BINDING_INDEX,
                &su.buffer,
            );
        }
        if let Some(mu) = &self.midi_uniform {
            sound_bind_group_factory.add_entry(
                crate::uniforms::midi_uniform::MidiUniform::BINDING_INDEX,
                &mu.buffer,
            );
        }
        let (sound_bind_group_layout, _) = sound_bind_group_factory.create(&self.device, "sound");

        // Build bind group layouts array (same as original)
        let mut bind_group_layouts = vec![&uniform_bind_group_layout, &device_bind_group_layout];
        if let Some(layout) = &sound_bind_group_layout {
            bind_group_layouts.push(&layout);
        }

        // Safely attempt to create new MultiPassPipeline with error handling
        // We use AssertUnwindSafe to bypass UnwindSafe requirements, as we're confident
        // that shader compilation errors won't violate memory safety
        log::info!("Attempting to create new MultiPassPipeline for hot reload...");

        let pipeline_result = panic::catch_unwind(AssertUnwindSafe(|| {
            crate::pipeline::MultiPassPipeline::new(
                &self.device,
                conf_dir,
                &self.config.pipeline,
                &self.surface_config,
                &bind_group_layouts,
            )
        }));

        match pipeline_result {
            Ok(new_pipeline) => {
                log::info!("Successfully created new MultiPassPipeline for hot reload");
                Ok(new_pipeline)
            }
            Err(_) => {
                log::error!(
                    "Shader compilation failed during hot reload - keeping existing pipeline"
                );
                Err("Shader compilation failed during hot reload".to_string())
            }
        }
    }

    fn determine_texture_type(&self, pass_index: usize) -> TextureType {
        if let Some(shader_config) = self.config.pipeline.get(pass_index) {
            if shader_config.ping_pong.unwrap_or(false) {
                TextureType::PingPong
            } else if shader_config.persistent.unwrap_or(false) {
                TextureType::Persistent
            } else {
                TextureType::Intermediate
            }
        } else {
            TextureType::Intermediate
        }
    }

    /// Helper method to create texture bind groups using BindGroupFactory
    fn create_texture_bind_group(
        &self,
        texture_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        label: &str,
    ) -> Option<wgpu::BindGroup> {
        if let Some(ref layout) = self.multi_pass_pipeline.texture_bind_group_layout {
            let mut factory = BindGroupFactory::new();
            factory.add_multipass_texture(texture_view, sampler);

            // Create bind group manually since we need to use the existing layout
            Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &factory.entries,
                label: Some(label),
            }))
        } else {
            None
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let final_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.surface_config.format.add_srgb_suffix()),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Update texture manager for new frame
        self.texture_manager.advance_frame();

        let pipeline_count = self.multi_pass_pipeline.pipeline_count();
        let is_multipass = self.multi_pass_pipeline.is_multi_pass();

        let has_persistent_textures =
            (0..pipeline_count).any(|i| self.determine_texture_type(i) == TextureType::Persistent);
        let has_ping_pong_textures =
            (0..pipeline_count).any(|i| self.determine_texture_type(i) == TextureType::PingPong);

        // Enter multi-pass rendering mode if:
        // 1. Multiple pipelines in sequence (traditional multi-pass)
        // 2. Any persistent textures (single-pass but needs state preservation)
        // 3. Any ping-pong textures (single-pass but needs double-buffering)
        if (is_multipass && pipeline_count > 1) || has_persistent_textures || has_ping_pong_textures
        {
            // Pre-create all textures based on shader configuration to avoid borrowing conflicts
            for i in 0..pipeline_count {
                let texture_type = self.determine_texture_type(i);
                // Skip texture creation for final pass unless it's persistent or ping-pong
                if i == pipeline_count - 1
                    && texture_type != TextureType::Persistent
                    && texture_type != TextureType::PingPong
                {
                    continue;
                }
                match texture_type {
                    TextureType::Intermediate => {
                        let _ = self
                            .texture_manager
                            .get_or_create_intermediate_texture(&self.device, i);
                    }
                    TextureType::PingPong => {
                        let _ = self
                            .texture_manager
                            .get_or_create_ping_pong_texture(&self.device, i);
                    }
                    TextureType::Persistent => {
                        let _ = self
                            .texture_manager
                            .get_or_create_persistent_texture(&self.device, i);
                    }
                }
            }

            // Multi-pass rendering for intermediate textures
            for pass_index in 0..pipeline_count {
                let current_texture_type = self.determine_texture_type(pass_index);
                let is_final_pass = pass_index == pipeline_count - 1
                    && current_texture_type != TextureType::Persistent
                    && current_texture_type != TextureType::PingPong;

                // Get render target view for this pass
                let render_target_view = if is_final_pass {
                    &final_view
                } else {
                    // Get pre-created texture based on shader configuration
                    let texture_type = self.determine_texture_type(pass_index);
                    match texture_type {
                        TextureType::Intermediate => self
                            .texture_manager
                            .get_intermediate_render_target(pass_index)
                            .expect("Intermediate texture should exist"),
                        TextureType::PingPong => self
                            .texture_manager
                            .get_ping_pong_render_target(pass_index)
                            .expect("Ping-pong texture should exist"),
                        TextureType::Persistent => self
                            .texture_manager
                            .get_persistent_render_target(pass_index)
                            .expect("Persistent texture should exist"),
                    }
                };

                // Create texture bind group for input (from previous pass or previous frame)
                let current_texture_type = self.determine_texture_type(pass_index);
                log::debug!(
                    "Pass {}: texture_type = {:?}, frame = {}",
                    pass_index,
                    current_texture_type,
                    self.texture_manager.current_frame
                );
                let texture_bind_group = if pass_index > 0
                    || current_texture_type == TextureType::Persistent
                    || current_texture_type == TextureType::PingPong
                {
                    let input_texture_view = if (current_texture_type == TextureType::Persistent
                        || current_texture_type == TextureType::PingPong)
                        && pass_index == 0
                    {
                        // For persistent/ping-pong textures on first pass, read from previous frame using double-buffering
                        match current_texture_type {
                            TextureType::Persistent => {
                                let textures = self
                                    .texture_manager
                                    .persistent_textures
                                    .get(&pass_index)
                                    .unwrap();
                                let read_index =
                                    ((self.texture_manager.current_frame + 1) % 2) as usize; // Read from previous frame
                                log::debug!(
                                    "Persistent texture input: frame={}, read_index={}",
                                    self.texture_manager.current_frame,
                                    read_index
                                );
                                &textures[read_index].1
                            }
                            TextureType::PingPong => {
                                // For ping-pong buffers, read from the PREVIOUS frame's texture
                                // If current_frame is even (0, 2, 4...), we're writing to buffer 0, so read from buffer 1
                                // If current_frame is odd (1, 3, 5...), we're writing to buffer 1, so read from buffer 0
                                let textures = self
                                    .texture_manager
                                    .ping_pong_textures
                                    .get(&pass_index)
                                    .unwrap();
                                let read_index =
                                    ((self.texture_manager.current_frame + 1) % 2) as usize; // Read from previous frame
                                log::debug!(
                                    "Ping-pong texture input: frame={}, read_index={}, write_index={}",
                                    self.texture_manager.current_frame,
                                    read_index,
                                    self.texture_manager.current_frame % 2
                                );
                                &textures[read_index].1
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        // For multi-pass, read from previous pass
                        let prev_pass_index = pass_index - 1;
                        let prev_texture_type = self.determine_texture_type(prev_pass_index);
                        match prev_texture_type {
                            TextureType::Intermediate => self
                                .texture_manager
                                .get_intermediate_render_target(prev_pass_index)
                                .expect("Previous pass intermediate texture should exist"),
                            TextureType::PingPong => self
                                .texture_manager
                                .get_ping_pong_render_target(prev_pass_index)
                                .expect("Previous pass ping-pong texture should exist"),
                            TextureType::Persistent => self
                                .texture_manager
                                .get_persistent_render_target(prev_pass_index)
                                .expect("Previous pass persistent texture should exist"),
                        }
                    };
                    let sampler = &self.texture_manager.sampler;

                    log::debug!(
                        "Creating texture bind group for pass {} ({:?})",
                        pass_index,
                        current_texture_type
                    );

                    self.create_texture_bind_group(
                        input_texture_view,
                        sampler,
                        &format!("Pass {} Texture Bind Group", pass_index),
                    )
                } else {
                    None
                };

                // Execute render pass
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some(&format!("Render Pass {}", pass_index)),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: render_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: if current_texture_type == TextureType::Persistent {
                                    // For persistent textures, clear on first frame, load thereafter
                                    let is_initialized = self
                                        .texture_manager
                                        .is_persistent_texture_initialized(pass_index);
                                    log::info!(
                                        "Persistent texture {} LoadOp: {} (initialized: {})",
                                        pass_index,
                                        if is_initialized { "Load" } else { "Clear" },
                                        is_initialized
                                    );
                                    if is_initialized {
                                        wgpu::LoadOp::Load
                                    } else {
                                        // Clear with black for first frame so we have something to read from
                                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                                    }
                                } else {
                                    wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                                },
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });

                    if let Some(pipeline) = self.multi_pass_pipeline.get_pipeline(pass_index) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                        render_pass.set_bind_group(1, &self.device_bind_group, &[]);

                        if let Some(sound_bind_group) = &self.sound_bind_group {
                            render_pass.set_bind_group(2, sound_bind_group, &[]);
                        } else if pass_index > 0
                            || current_texture_type == TextureType::Persistent
                            || current_texture_type == TextureType::PingPong
                        {
                            // Create empty bind group for Group 2 if needed for multipass or persistent texture
                            if let Some(ref empty_layout) =
                                self.multi_pass_pipeline.empty_bind_group_layout
                            {
                                let empty_bind_group =
                                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                        layout: empty_layout,
                                        entries: &[],
                                        label: Some("Empty Group 2 Bind Group"),
                                    });
                                render_pass.set_bind_group(2, &empty_bind_group, &[]);
                            }
                        }

                        // Set texture bind group for multi-pass input (Group 3)
                        if let Some(ref bind_group) = texture_bind_group {
                            log::debug!("Setting texture bind group for pass {}", pass_index);
                            render_pass.set_bind_group(3, bind_group, &[]);
                        } else {
                            log::debug!("No texture bind group for pass {}", pass_index);
                        }

                        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        render_pass.set_index_buffer(
                            self.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                    }
                }

                // Mark texture as initialized after first render
                match current_texture_type {
                    TextureType::Persistent => {
                        log::debug!("Marking persistent texture {} as initialized", pass_index);
                        self.texture_manager
                            .mark_persistent_texture_initialized(pass_index);
                    }
                    TextureType::PingPong => {
                        log::debug!("Marking ping-pong texture {} as initialized", pass_index);
                        self.texture_manager
                            .mark_ping_pong_texture_initialized(pass_index);
                    }
                    _ => {}
                }
            }

            // Copy persistent and ping-pong textures to screen
            // Both persistent and ping-pong textures render to intermediate textures,
            // so we need to copy their final output to the screen for display
            for pass_index in 0..pipeline_count {
                let texture_type = self.determine_texture_type(pass_index);
                if texture_type == TextureType::Persistent || texture_type == TextureType::PingPong
                {
                    // Copy texture to screen using a simple copy pass
                    let _texture_view = match texture_type {
                        TextureType::Persistent => self
                            .texture_manager
                            .get_persistent_render_target(pass_index)
                            .expect("Persistent texture should exist"),
                        TextureType::PingPong => self
                            .texture_manager
                            .get_ping_pong_render_target(pass_index)
                            .expect("Ping-pong texture should exist"),
                        _ => unreachable!(),
                    };

                    let mut copy_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some(&format!("Copy {:?} to Screen", texture_type)),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &final_view,
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

                    // Use a simple texture copy or render the persistent texture directly
                    if let Some(pipeline) = self.multi_pass_pipeline.get_pipeline(pass_index) {
                        copy_pass.set_pipeline(pipeline);
                        copy_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                        copy_pass.set_bind_group(1, &self.device_bind_group, &[]);

                        if let Some(sound_bind_group) = &self.sound_bind_group {
                            copy_pass.set_bind_group(2, sound_bind_group, &[]);
                        } else {
                            if let Some(ref empty_layout) =
                                self.multi_pass_pipeline.empty_bind_group_layout
                            {
                                let empty_bind_group =
                                    self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                                        layout: empty_layout,
                                        entries: &[],
                                        label: Some("Empty Bind Group for Group 2"),
                                    });
                                copy_pass.set_bind_group(2, &empty_bind_group, &[]);
                            }
                        }

                        // Bind the texture for reading
                        let read_index = ((self.texture_manager.current_frame + 1) % 2) as usize;
                        let texture_view_ref = match texture_type {
                            TextureType::Persistent => {
                                let textures = self
                                    .texture_manager
                                    .persistent_textures
                                    .get(&pass_index)
                                    .unwrap();
                                &textures[read_index].1
                            }
                            TextureType::PingPong => {
                                let textures = self
                                    .texture_manager
                                    .ping_pong_textures
                                    .get(&pass_index)
                                    .unwrap();
                                &textures[read_index].1
                            }
                            _ => unreachable!(),
                        };

                        let texture_bind_group = self
                            .create_texture_bind_group(
                                texture_view_ref,
                                &self.texture_manager.sampler,
                                &format!("Copy {:?} Texture Bind Group", texture_type),
                            )
                            .expect(
                                "Should be able to create texture bind group for copy operation",
                            );
                        copy_pass.set_bind_group(3, &texture_bind_group, &[]);

                        copy_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        copy_pass.set_index_buffer(
                            self.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        copy_pass.draw_indexed(0..self.num_indices, 0, 0..1);
                    }
                }
            }
        } else {
            // Single-pass rendering (backward compatibility)
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Single Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &final_view,
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

            if let Some(pipeline) = self.multi_pass_pipeline.get_pipeline(0) {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                render_pass.set_bind_group(1, &self.device_bind_group, &[]);

                if let Some(sound_bind_group) = &self.sound_bind_group {
                    render_pass.set_bind_group(2, sound_bind_group, &[]);
                }

                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture_manager::TextureType;

    /// Helper function to create a test device for unit tests
    fn create_test_device() -> wgpu::Device {
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
                .0
        })
    }

    /// Test the determine_texture_type logic that will be extracted
    #[test]
    fn test_determine_texture_type_logic() {
        // Test ping-pong texture detection
        let ping_pong_config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Game of Life"
entry_point = "fs_main"
file = "life.wgsl"
ping_pong = true
"#;
        let ping_pong_config: Config = toml::from_str(ping_pong_config_content).unwrap();

        // Mock the logic from State::determine_texture_type
        let determine_texture_type = |config: &Config, pass_index: usize| -> TextureType {
            if pass_index < config.pipeline.len() {
                let shader_config = &config.pipeline[pass_index];
                if shader_config.persistent.unwrap_or(false) {
                    TextureType::Persistent
                } else if shader_config.ping_pong.unwrap_or(false) {
                    TextureType::PingPong
                } else {
                    TextureType::Intermediate
                }
            } else {
                TextureType::Intermediate
            }
        };

        assert_eq!(
            determine_texture_type(&ping_pong_config, 0),
            TextureType::PingPong
        );

        // Test persistent texture detection
        let persistent_config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Persistent Effect"
entry_point = "fs_main"
file = "persistent.wgsl"
persistent = true
"#;
        let persistent_config: Config = toml::from_str(persistent_config_content).unwrap();
        assert_eq!(
            determine_texture_type(&persistent_config, 0),
            TextureType::Persistent
        );

        // Test intermediate texture (default)
        let simple_config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Simple"
entry_point = "fs_main"
file = "simple.wgsl"
"#;
        let simple_config: Config = toml::from_str(simple_config_content).unwrap();
        assert_eq!(
            determine_texture_type(&simple_config, 0),
            TextureType::Intermediate
        );
    }

    /// Test render requirements analysis that will be extracted
    #[test]
    fn test_render_requirements_analysis() {
        // Test the analysis logic without creating actual pipelines
        let simple_config_content = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Test Shader"
entry_point = "fs_main"
file = "test.wgsl"
"#;
        let config: Config = toml::from_str(simple_config_content).unwrap();

        // Mock the analysis logic from State::render without MultiPassPipeline dependency
        let analyze_requirements = |config: &Config| -> (bool, bool, bool) {
            let pipeline_count = config.pipeline.len();

            let has_persistent =
                (0..pipeline_count).any(|i| config.pipeline[i].persistent.unwrap_or(false));

            let has_ping_pong =
                (0..pipeline_count).any(|i| config.pipeline[i].ping_pong.unwrap_or(false));

            // For testing purposes, simulate is_multipass based on config
            let is_multipass = pipeline_count > 1 || has_persistent || has_ping_pong;
            let requires_multipass =
                (is_multipass && pipeline_count > 1) || has_persistent || has_ping_pong;

            (has_persistent, has_ping_pong, requires_multipass)
        };

        let (has_persistent, has_ping_pong, requires_multipass) = analyze_requirements(&config);

        // Single simple pipeline should not require multipass
        assert!(!has_persistent);
        assert!(!has_ping_pong);
        assert!(!requires_multipass);
    }

    /// Test texture creation skip logic for final pass
    #[test]
    fn test_texture_creation_skip_logic() {
        // Mock the skip logic from State::render texture creation loop
        let should_skip_texture_creation =
            |pass_index: usize, pipeline_count: usize, texture_type: TextureType| -> bool {
                pass_index == pipeline_count - 1
                    && texture_type != TextureType::Persistent
                    && texture_type != TextureType::PingPong
            };

        // Final pass with intermediate texture should be skipped
        assert!(should_skip_texture_creation(
            2,
            3,
            TextureType::Intermediate
        ));

        // Final pass with persistent texture should NOT be skipped
        assert!(!should_skip_texture_creation(2, 3, TextureType::Persistent));

        // Final pass with ping-pong texture should NOT be skipped
        assert!(!should_skip_texture_creation(2, 3, TextureType::PingPong));

        // Non-final pass should never be skipped
        assert!(!should_skip_texture_creation(
            0,
            3,
            TextureType::Intermediate
        ));
        assert!(!should_skip_texture_creation(
            1,
            3,
            TextureType::Intermediate
        ));
    }

    /// Test final pass detection logic
    #[test]
    fn test_final_pass_detection() {
        // Mock the final pass detection from State::render multipass loop
        let is_final_pass =
            |pass_index: usize, pipeline_count: usize, texture_type: TextureType| -> bool {
                pass_index == pipeline_count - 1
                    && texture_type != TextureType::Persistent
                    && texture_type != TextureType::PingPong
            };

        // Final pass with intermediate texture is a final pass
        assert!(is_final_pass(2, 3, TextureType::Intermediate));

        // Final pass with persistent texture is NOT a final pass (needs texture)
        assert!(!is_final_pass(2, 3, TextureType::Persistent));

        // Final pass with ping-pong texture is NOT a final pass (needs texture)
        assert!(!is_final_pass(2, 3, TextureType::PingPong));

        // Non-final pass is never a final pass
        assert!(!is_final_pass(0, 3, TextureType::Intermediate));
        assert!(!is_final_pass(1, 3, TextureType::Intermediate));
    }

    /// Test render target selection logic
    #[test]
    fn test_render_target_selection() {
        // Mock the render target selection logic from State::render
        enum RenderTarget {
            FinalView,
            IntermediateTexture,
            PingPongTexture,
            PersistentTexture,
        }

        let select_render_target =
            |is_final_pass: bool, texture_type: TextureType| -> RenderTarget {
                if is_final_pass {
                    RenderTarget::FinalView
                } else {
                    match texture_type {
                        TextureType::Intermediate => RenderTarget::IntermediateTexture,
                        TextureType::PingPong => RenderTarget::PingPongTexture,
                        TextureType::Persistent => RenderTarget::PersistentTexture,
                    }
                }
            };

        // Final pass should use final view
        match select_render_target(true, TextureType::Intermediate) {
            RenderTarget::FinalView => {}
            _ => panic!("Final pass should use final view"),
        }

        // Non-final intermediate pass should use intermediate texture
        match select_render_target(false, TextureType::Intermediate) {
            RenderTarget::IntermediateTexture => {}
            _ => panic!("Intermediate pass should use intermediate texture"),
        }

        // Non-final ping-pong pass should use ping-pong texture
        match select_render_target(false, TextureType::PingPong) {
            RenderTarget::PingPongTexture => {}
            _ => panic!("Ping-pong pass should use ping-pong texture"),
        }

        // Non-final persistent pass should use persistent texture
        match select_render_target(false, TextureType::Persistent) {
            RenderTarget::PersistentTexture => {}
            _ => panic!("Persistent pass should use persistent texture"),
        }
    }

    /// Test frame calculation logic for texture reading
    #[test]
    fn test_frame_calculation_logic() {
        // Mock the frame calculation logic from State::render
        let calculate_read_index = |current_frame: u32| -> usize {
            ((current_frame + 1) % 2) as usize // Read from previous frame
        };

        // Test frame index calculation
        assert_eq!(calculate_read_index(0), 1); // Frame 0 reads from index 1
        assert_eq!(calculate_read_index(1), 0); // Frame 1 reads from index 0
        assert_eq!(calculate_read_index(2), 1); // Frame 2 reads from index 1
        assert_eq!(calculate_read_index(3), 0); // Frame 3 reads from index 0
    }

    /// Test multipass condition logic
    #[test]
    fn test_multipass_condition() {
        // Mock the multipass condition from State::render
        let should_use_multipass = |is_multipass: bool,
                                    pipeline_count: usize,
                                    has_persistent: bool,
                                    has_ping_pong: bool|
         -> bool {
            (is_multipass && pipeline_count > 1) || has_persistent || has_ping_pong
        };

        // Multi-pass with multiple pipelines should use multipass
        assert!(should_use_multipass(true, 2, false, false));

        // Multi-pass with single pipeline should NOT use multipass (unless special textures)
        assert!(!should_use_multipass(true, 1, false, false));

        // Single pass with persistent textures should use multipass
        assert!(should_use_multipass(false, 1, true, false));

        // Single pass with ping-pong textures should use multipass
        assert!(should_use_multipass(false, 1, false, true));

        // Single pass without special textures should not use multipass
        assert!(!should_use_multipass(false, 1, false, false));
    }
}
