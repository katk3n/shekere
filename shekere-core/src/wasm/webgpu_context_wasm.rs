use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use crate::console_log;

pub struct WebGpuContextWasm {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
}

impl WebGpuContextWasm {
    pub async fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        console_log!("Initializing WebGPU context for WASM...");

        console_log!("Creating wgpu instance...");

        // Create the wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        console_log!("wgpu instance created, creating surface from canvas...");
        console_log!("Canvas info: tag={}, width={}, height={}",
            canvas.tag_name(), canvas.width(), canvas.height());

        // Create surface from canvas with proper error handling
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| {
                let error_msg = format!("Failed to create surface: {:?}", e);
                console_log!("❌ Surface creation failed: {}", error_msg);
                console_log!("This could be because:");
                console_log!("1. WebGPU is not enabled in the browser");
                console_log!("2. Canvas is already in use by another context");
                console_log!("3. Hardware acceleration is disabled");
                console_log!("4. Browser does not support WebGPU");
                JsValue::from_str(&error_msg)
            })?;

        console_log!("Surface created successfully");

        console_log!("Requesting WebGPU adapter with high-performance preference...");

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| {
                console_log!("❌ Failed to find a suitable WebGPU adapter");
                console_log!("This could be because:");
                console_log!("1. WebGPU is not enabled in browser flags");
                console_log!("2. GPU drivers are not compatible");
                console_log!("3. Browser does not support WebGPU");
                console_log!("4. wgpu backend mismatch with browser WebGPU");
                JsValue::from_str("Failed to find a suitable WebGPU adapter")
            })?;

        let adapter_info = adapter.get_info();
        console_log!("✅ WebGPU adapter found successfully:");
        console_log!("   Name: {}", adapter_info.name);
        console_log!("   Vendor: {}", adapter_info.vendor);
        console_log!("   Device: {}", adapter_info.device);
        console_log!("   Backend: {:?}", adapter_info.backend);

        console_log!("Requesting WebGPU device and queue...");

        // Request device with default limits (let adapter decide)
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("shekere-wasm-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(), // Use adapter's default limits
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| {
                let error_msg = format!("Failed to request device: {:?}", e);
                console_log!("❌ {}", error_msg);
                JsValue::from_str(&error_msg)
            })?;

        console_log!("✅ WebGPU device and queue created successfully");

        // Get canvas dimensions
        let width = canvas.width();
        let height = canvas.height();

        console_log!("Canvas dimensions: {}x{}", width, height);

        if width == 0 || height == 0 {
            let error_msg = format!("Invalid canvas dimensions: {}x{}", width, height);
            console_log!("❌ {}", error_msg);
            return Err(JsValue::from_str(&error_msg));
        }

        console_log!("Configuring surface...");

        // Configure the surface
        let surface_caps = surface.get_capabilities(&adapter);
        console_log!("Surface capabilities:");
        console_log!("   Formats: {} available", surface_caps.formats.len());
        console_log!("   Present modes: {} available", surface_caps.present_modes.len());
        console_log!("   Alpha modes: {} available", surface_caps.alpha_modes.len());

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        console_log!("Selected surface format: {:?}", surface_format);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        console_log!("Configuring surface with config...");
        surface.configure(&device, &surface_config);

        console_log!("✅ WebGPU context initialized successfully");

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
        })
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            self.surface_config.width = new_width;
            self.surface_config.height = new_height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }
}