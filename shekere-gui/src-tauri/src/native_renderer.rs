use std::sync::Arc;
use winit::window::Window;

/// Native WebGPU surface renderer that bypasses data transfer overhead
/// This provides CLI-level performance by rendering directly to native window surface
pub struct NativeRenderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    window_size: (u32, u32),
    renderer: Option<shekere_core::Renderer<'static>>, // Optional to handle initialization
    config: Box<shekere_core::Config>,                 // Owned config for static lifetime
    config_dir: std::path::PathBuf,
}

impl NativeRenderer {
    /// Create a new native renderer directly attached to a winit window
    pub async fn new(
        window: Arc<Window>,
        config: shekere_core::Config,
        config_dir: std::path::PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Creating native WebGPU surface renderer");

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        // Create WebGPU instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        // Create surface from window - this is the key difference from texture rendering
        let surface = instance.create_surface(window.clone())?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance, // Request high performance
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("Failed to find suitable adapter")?;

        // Request device and queue
        let (device, _queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: Some("Native Renderer Device"),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;

        // Configure surface
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
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo, // VSync for smooth rendering
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        // Store config in a Box for 'static lifetime
        let config_boxed = Box::new(config);

        log::info!(
            "Native renderer created successfully: {}x{}, format: {:?}",
            width,
            height,
            surface_format
        );

        Ok(Self {
            surface,
            surface_config,
            window_size: (width, height),
            renderer: None, // Will be initialized lazily when first rendering
            config: config_boxed,
            config_dir,
        })
    }

    /// Render a frame directly to the window surface - bypassing all data transfer
    pub async fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let frame_start = std::time::Instant::now();

        // Initialize renderer lazily if not already done
        if self.renderer.is_none() {
            log::info!("Initializing renderer lazily on first render");

            // Create WebGPU context compatible with the surface
            let webgpu_context = shekere_core::WebGpuContext::new_with_surface(&self.surface)
                .await
                .map_err(|e| format!("Failed to create WebGPU context: {}", e))?;

            // Use Box::leak to create a 'static reference to the config
            // This is safe because the config will live as long as the NativeRenderer
            let config_ref: &'static shekere_core::Config = Box::leak(self.config.clone());

            // Create the renderer with the static config reference
            let renderer = shekere_core::Renderer::new(
                webgpu_context,
                config_ref,
                &self.config_dir,
                self.window_size.0,
                self.window_size.1,
            )
            .await
            .map_err(|e| format!("Failed to create renderer: {}", e))?;

            self.renderer = Some(renderer);
        }

        // Get the renderer (guaranteed to exist now)
        let renderer = self.renderer.as_mut().unwrap();

        // Update renderer (time, uniforms, etc.) - using stored renderer!
        renderer.update(0.016); // ~60 FPS delta time

        // Render directly to surface using stored renderer - NO data transfer or recreation!
        // The render_to_surface method handles get_current_texture internally
        renderer
            .render_to_surface(&self.surface, &self.surface_config)
            .map_err(|e| format!("Surface render failed: {}", e))?;

        let render_time = frame_start.elapsed().as_micros();
        log::debug!("Native frame render: {}μs", render_time);

        Ok(())
    }

    /// Handle window resize
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        let width = new_size.width.max(1);
        let height = new_size.height.max(1);

        log::info!("Resizing native renderer: {}x{}", width, height);

        // Update surface configuration
        self.surface_config.width = width;
        self.surface_config.height = height;

        // Apply the new configuration to the surface if we have a renderer
        if let Some(renderer) = &self.renderer {
            self.surface
                .configure(renderer.get_device(), &self.surface_config);
        }

        // Update renderer size so uniforms are updated correctly
        if let Some(renderer) = &mut self.renderer {
            renderer.update_size(width, height);
        }

        // Update stored window size
        self.window_size = (width, height);
    }

    /// Handle mouse input directly using stored renderer
    pub fn handle_mouse_input(&mut self, x: f64, y: f64) {
        // Handle mouse input immediately using the stored renderer if available
        if let Some(renderer) = &mut self.renderer {
            renderer.handle_mouse_input(x, y);
            log::debug!("Mouse input processed: ({}, {})", x, y);
        }
    }

    /// Get current window size
    pub fn get_size(&self) -> (u32, u32) {
        self.window_size
    }
}
