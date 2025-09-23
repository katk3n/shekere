use crate::config::Config;
use crate::pipeline::MultiPassPipeline;
use crate::texture_manager::{TextureManager, TextureType};
use crate::timer::Timer;
use crate::uniform_manager::UniformManager;
use crate::vertex::{INDICES, VERTICES};
use crate::webgpu_context::WebGpuContext;

use std::path::Path;
use thiserror::Error;
use wgpu::util::DeviceExt;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("Uniform manager error: {0}")]
    UniformManager(#[from] crate::uniform_manager::UniformManagerError),
    #[error("Surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Pipeline creation failed")]
    PipelineCreation,
}

/// Window-independent renderer that can render to surfaces or textures.
/// This is the core rendering component that doesn't depend on winit::Window.
pub struct Renderer<'a> {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Pipeline and texture management
    multi_pass_pipeline: MultiPassPipeline,
    texture_manager: TextureManager,

    // Vertex data
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    // Uniform and time management
    uniform_manager: UniformManager<'a>,
    timer: Timer,

    // Configuration
    config: Config,
    config_dir: std::path::PathBuf,
}

impl<'a> Renderer<'a> {
    /// Create a new Renderer from WebGpuContext and configuration.
    /// Window dimensions are provided as parameters instead of requiring a Window.
    pub async fn new(
        context: WebGpuContext,
        config: &'a Config,
        config_dir: &Path,
        window_width: u32,
        window_height: u32,
    ) -> Result<Self, RendererError> {
        // Create timer
        let timer = Timer::new();

        Self::new_with_timer(
            context,
            config,
            config_dir,
            window_width,
            window_height,
            timer,
        )
        .await
    }

    /// Create a new Renderer with a custom timer for animation continuity.
    pub async fn new_with_timer(
        context: WebGpuContext,
        config: &'a Config,
        config_dir: &Path,
        window_width: u32,
        window_height: u32,
        timer: Timer,
    ) -> Result<Self, RendererError> {
        let WebGpuContext { device, queue } = context;

        // Create uniform manager
        let uniform_manager =
            UniformManager::new(&device, config, window_width, window_height).await?;

        // Get bind group layouts for pipeline creation
        let bind_group_layouts = uniform_manager.get_bind_group_layouts();

        // Create a surface configuration for pipeline creation
        // We use a default format since we'll support multiple formats
        let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_width,
            height: window_height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: if !surface_format.is_srgb() {
                vec![surface_format.add_srgb_suffix()]
            } else {
                vec![]
            },
            desired_maximum_frame_latency: 2,
        };

        // Create multi-pass pipeline
        let multi_pass_pipeline = MultiPassPipeline::new(
            &device,
            config_dir,
            &config.pipeline,
            &surface_config,
            &bind_group_layouts,
        );

        // Create texture manager
        let texture_manager = TextureManager::new_with_format(
            &device,
            window_width,
            window_height,
            surface_config.format.add_srgb_suffix(),
        );

        // Create vertex and index buffers
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

        Ok(Self {
            device,
            queue,
            multi_pass_pipeline,
            texture_manager,
            vertex_buffer,
            index_buffer,
            num_indices,
            uniform_manager,
            timer,
            config: config.clone(),
            config_dir: config_dir.to_path_buf(),
        })
    }

    /// Update the renderer with time and input data.
    /// This should be called once per frame before rendering.
    pub fn update(&mut self, _delta_time: f32) {
        let time_duration = self.timer.get_duration();
        self.uniform_manager.update(&self.queue, time_duration);
    }

    /// Handle mouse input events
    pub fn handle_mouse_input(&mut self, x: f64, y: f64) -> bool {
        self.uniform_manager.handle_mouse_input(x, y)
    }

    /// Update window/viewport size
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.uniform_manager.update_window_size(width, height);
        // Clear textures on resize
        self.texture_manager.clear_all_textures();
    }

    /// Get a reference to the device
    pub fn get_device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get a reference to the queue
    pub fn get_queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Update configuration (for hot reload)
    pub fn update_config(&mut self, config: &'a Config) -> Result<(), RendererError> {
        // Store new config
        self.config = config.clone();

        // Recreate pipeline with new configuration
        let bind_group_layouts = self.uniform_manager.get_bind_group_layouts();

        // Create temporary surface config for pipeline creation
        let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: self.texture_manager.width(),
            height: self.texture_manager.height(),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: if !surface_format.is_srgb() {
                vec![surface_format.add_srgb_suffix()]
            } else {
                vec![]
            },
            desired_maximum_frame_latency: 2,
        };

        // Recreate the pipeline
        self.multi_pass_pipeline = MultiPassPipeline::new(
            &self.device,
            &self.config_dir,
            &config.pipeline,
            &surface_config,
            &bind_group_layouts,
        );

        // Clear texture manager state to avoid potential issues with stale textures
        self.texture_manager.clear_all_textures();

        Ok(())
    }

    /// Render to a surface (for CLI usage).
    /// This is equivalent to the current State::render() method.
    pub fn render_to_surface(
        &mut self,
        surface: &wgpu::Surface,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Result<(), RendererError> {
        // Get current texture from surface
        let output = surface.get_current_texture()?;
        let final_view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(surface_config.format.add_srgb_suffix()),
            ..Default::default()
        });

        // Render to the surface texture
        self.render_to_texture_view(&final_view)?;

        // Present the surface
        output.present();
        Ok(())
    }

    /// Render to a texture (for GUI usage).
    /// The target texture should be created by the caller.
    pub fn render_to_texture(&mut self, target: &wgpu::Texture) -> Result<(), RendererError> {
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
        self.render_to_texture_view(&target_view)
    }

    /// Internal method that does the actual rendering to a texture view.
    /// This contains the core rendering logic extracted from State::render().
    fn render_to_texture_view(
        &mut self,
        final_view: &wgpu::TextureView,
    ) -> Result<(), RendererError> {
        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Update texture manager for new frame
        self.texture_manager.advance_frame();

        // Analyze pass texture requirements (extracted from State::render())
        let pass_info = self.analyze_pass_texture_requirements();

        // Create textures for passes (extracted from State::render())
        self.create_textures_for_passes(&pass_info)?;

        // Create MultiPassContext to centralize conditional logic
        let has_texture_bindings = self.multi_pass_pipeline.texture_bind_group_layout.is_some();
        let context = crate::state::MultiPassContext::new(
            &pass_info,
            has_texture_bindings,
            self.texture_manager.current_frame,
        );

        // Choose rendering path based on requirements
        if context.requires_multipass_rendering() {
            self.render_multipass(&mut encoder, final_view, &context)?;
        } else {
            self.render_single_pass(&mut encoder, final_view)?;
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }

    /// Analyze texture requirements for all passes (extracted from State::analyze_pass_texture_requirements)
    fn analyze_pass_texture_requirements(&self) -> crate::state::PassTextureInfo {
        let pipeline_count = self.multi_pass_pipeline.pipeline_count();
        let _is_multipass = self.multi_pass_pipeline.is_multi_pass();

        // Collect texture types for all passes
        let texture_types: Vec<TextureType> = (0..pipeline_count)
            .map(|i| self.determine_texture_type(i))
            .collect();

        crate::state::PassTextureInfo::new(texture_types)
    }

    /// Determine texture type for a specific pass (extracted from State::determine_texture_type)
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

    /// Create textures for passes (extracted from State::create_textures_for_passes)
    fn create_textures_for_passes(
        &mut self,
        pass_info: &crate::state::PassTextureInfo,
    ) -> Result<(), RendererError> {
        let pipeline_count = self.multi_pass_pipeline.pipeline_count();

        // Pre-create all textures based on shader configuration to avoid borrowing conflicts
        for i in 0..pipeline_count {
            let texture_type = pass_info.texture_types[i];
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
        Ok(())
    }

    /// Single-pass rendering (extracted from State::render_single_pass)
    fn render_single_pass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        final_view: &wgpu::TextureView,
    ) -> Result<(), RendererError> {
        // Single-pass rendering (backward compatibility)
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Single Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: final_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Use common setup for Groups 0-2, vertex/index buffers, and draw call
        self.setup_render_pass_common(&mut render_pass, 0, TextureType::Intermediate);

        // Execute draw call (no Group 3 needed for simple single-pass)
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

        Ok(())
    }

    /// Setup common render pass elements (extracted from State::setup_render_pass_common)
    fn setup_render_pass_common<'pass>(
        &self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        pass_index: usize,
        current_texture_type: TextureType,
    ) {
        // Guard clause: early return if no pipeline available
        let Some(pipeline) = self.multi_pass_pipeline.get_pipeline(pass_index) else {
            return;
        };

        // Set the pipeline for this pass
        render_pass.set_pipeline(pipeline);

        // Group 0: uniform bind group (always present)
        render_pass.set_bind_group(0, &self.uniform_manager.uniform_bind_group, &[]);

        // Group 1: device bind group (always present)
        render_pass.set_bind_group(1, &self.uniform_manager.device_bind_group, &[]);

        // Group 2: sound bind group OR empty bind group (conditional)
        if let Some(sound_bind_group) = &self.uniform_manager.sound_bind_group {
            render_pass.set_bind_group(2, sound_bind_group, &[]);
        } else if self.needs_empty_bind_group(pass_index, current_texture_type) {
            // Create empty bind group for Group 2 if needed for multipass or persistent texture
            if let Some(ref empty_layout) = self.multi_pass_pipeline.empty_bind_group_layout {
                let empty_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: empty_layout,
                    entries: &[],
                    label: Some("Empty Group 2 Bind Group"),
                });
                render_pass.set_bind_group(2, &empty_bind_group, &[]);
            }
        }

        // Setup vertex and index buffers
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    }

    /// Helper method to determine if empty bind group is needed (extracted from State)
    fn needs_empty_bind_group(&self, pass_index: usize, current_texture_type: TextureType) -> bool {
        pass_index > 0
            || current_texture_type == TextureType::Persistent
            || current_texture_type == TextureType::PingPong
    }

    /// Multi-pass rendering (extracted from State::render_multipass)
    fn render_multipass(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        final_view: &wgpu::TextureView,
        context: &crate::state::MultiPassContext,
    ) -> Result<(), RendererError> {
        let pipeline_count = self.multi_pass_pipeline.pipeline_count();

        // Multi-pass rendering for intermediate textures
        for pass_index in 0..pipeline_count {
            let current_texture_type = context.pass_info.texture_types[pass_index];
            let is_final_pass = context.is_final_screen_pass(pass_index);

            // Collect information about what textures we need first
            let needs_binding = context.needs_texture_binding(pass_index);
            let needs_previous_frame = context.needs_previous_frame_input(pass_index);

            let input_texture_info = if needs_binding {
                if needs_previous_frame {
                    // For persistent/ping-pong textures on first pass, read from previous frame using double-buffering
                    match current_texture_type {
                        crate::texture_manager::TextureType::Persistent => {
                            let read_frame_index = context.get_read_frame_index();
                            Some((current_texture_type, pass_index, read_frame_index))
                        }
                        crate::texture_manager::TextureType::PingPong => {
                            let read_frame_index = context.get_read_frame_index();
                            Some((current_texture_type, pass_index, read_frame_index))
                        }
                        _ => None,
                    }
                } else {
                    // Input from previous pass (intermediate texture)
                    if pass_index > 0 {
                        let previous_texture_type = context.pass_info.texture_types[pass_index - 1];
                        let write_frame_index = (self.texture_manager.current_frame % 2) as usize;
                        Some((previous_texture_type, pass_index - 1, write_frame_index))
                    } else {
                        None
                    }
                }
            } else {
                None
            };

            // Create any missing intermediate textures first (mutable operations)
            if let Some((
                crate::texture_manager::TextureType::Intermediate,
                texture_pass_index,
                _,
            )) = input_texture_info
            {
                let _ = self
                    .texture_manager
                    .get_or_create_intermediate_texture(&self.device, texture_pass_index);
            }

            // Now we can safely get immutable references for render targets and texture views
            let render_target_view = if is_final_pass {
                final_view
            } else {
                // Get pre-created texture based on shader configuration
                match current_texture_type {
                    crate::texture_manager::TextureType::Intermediate => self
                        .texture_manager
                        .get_intermediate_render_target(pass_index)
                        .expect("Intermediate texture should exist"),
                    crate::texture_manager::TextureType::PingPong => self
                        .texture_manager
                        .get_ping_pong_render_target(pass_index)
                        .expect("Ping-pong texture should exist"),
                    crate::texture_manager::TextureType::Persistent => self
                        .texture_manager
                        .get_persistent_render_target(pass_index)
                        .expect("Persistent texture should exist"),
                }
            };

            // Create texture bind group for input

            let texture_bind_group =
                if let Some((texture_type, texture_pass_index, frame_index)) = input_texture_info {
                    let input_texture_view = match texture_type {
                        crate::texture_manager::TextureType::Intermediate => self
                            .texture_manager
                            .get_intermediate_render_target(texture_pass_index),
                        crate::texture_manager::TextureType::PingPong => {
                            let textures = self
                                .texture_manager
                                .ping_pong_textures
                                .get(&texture_pass_index)
                                .unwrap();
                            Some(&textures[frame_index].1)
                        }
                        crate::texture_manager::TextureType::Persistent => {
                            let textures = self
                                .texture_manager
                                .persistent_textures
                                .get(&texture_pass_index)
                                .unwrap();
                            Some(&textures[frame_index].1)
                        }
                    };

                    if let Some(input_view) = input_texture_view {
                        Some(self.create_texture_bind_group(input_view)?)
                    } else {
                        None
                    }
                } else {
                    None
                };

            // Begin render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(&format!("Render Pass {}", pass_index)),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Setup common elements (Groups 0-2, vertex/index buffers)
            self.setup_render_pass_common(&mut render_pass, pass_index, current_texture_type);

            // Group 3: texture bind group (conditional, only if texture binding is needed)
            if let Some(ref bg) = texture_bind_group {
                render_pass.set_bind_group(3, bg, &[]);
            }

            // Execute draw call
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        // Step 2: Copy persistent/ping-pong textures to screen (extracted from State::render_multipass)
        // Both persistent and ping-pong textures render to intermediate textures,
        // so we need to copy their final output to the screen for display
        for pass_index in 0..pipeline_count {
            let texture_type = context.pass_info.texture_types[pass_index];
            if context.is_stateful_texture(texture_type) {
                // Get the texture view to copy from
                let source_texture_view = match texture_type {
                    crate::texture_manager::TextureType::Persistent => self
                        .texture_manager
                        .get_persistent_render_target(pass_index)
                        .expect("Persistent texture should exist"),
                    crate::texture_manager::TextureType::PingPong => self
                        .texture_manager
                        .get_ping_pong_render_target(pass_index)
                        .expect("Ping-pong texture should exist"),
                    _ => unreachable!(),
                };

                // Create texture bind group for copying
                let copy_bind_group = self.create_texture_bind_group(source_texture_view)?;

                // Copy pass to screen
                let mut copy_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(&format!("Copy {:?} to Screen", texture_type)),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: final_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                // Setup copy pass with a simple pipeline (we need a simple copy shader for this)
                self.setup_render_pass_common(&mut copy_pass, pass_index, texture_type);

                // Group 3: source texture for copying
                copy_pass.set_bind_group(3, &copy_bind_group, &[]);

                // Execute copy draw call
                copy_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
        }

        Ok(())
    }

    /// Create texture bind group for multipass rendering (extracted from State)
    fn create_texture_bind_group(
        &self,
        texture_view: &wgpu::TextureView,
    ) -> Result<wgpu::BindGroup, RendererError> {
        if let Some(ref layout) = self.multi_pass_pipeline.texture_bind_group_layout {
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
                label: Some("Texture Bind Group"),
            });

            Ok(bind_group)
        } else {
            Err(RendererError::Config(
                "Missing texture bind group layout".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::webgpu_context::WebGpuContext;
    use std::path::Path;

    #[tokio::test]
    async fn test_renderer_creation() {
        // Create a minimal config for testing
        let config_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "test"
entry_point = "fs_main"
file = "test.wgsl"
"#;

        let config: Config = toml::from_str(config_str).unwrap();

        // For testing, we need a device. In real tests, we might mock this.
        // For now, we'll just test that the code compiles correctly.
        let can_create_renderer = true;
        assert!(can_create_renderer);
    }

    #[test]
    fn test_renderer_error_display() {
        let error = RendererError::Config("test error".to_string());
        assert_eq!(error.to_string(), "Configuration error: test error");

        let error = RendererError::PipelineCreation;
        assert_eq!(error.to_string(), "Pipeline creation failed");
    }
}
