use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};
use crate::console_log;

pub struct SimpleWebGl2Renderer {
    canvas: HtmlCanvasElement,
    gl: WebGl2RenderingContext,
    width: u32,
    height: u32,
    time_start: f64,
    mouse_x: f32,
    mouse_y: f32,
    is_initialized: bool,
    shader_content: Option<String>,
    shader_hash: u32,
}

impl SimpleWebGl2Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        console_log!("Creating simple WebGL2 renderer...");

        let gl = canvas
            .get_context("webgl2")?
            .ok_or("WebGL2 not supported")?
            .dyn_into::<WebGl2RenderingContext>()?;

        console_log!("✅ WebGL2 context created successfully");

        let width = canvas.width();
        let height = canvas.height();

        if width == 0 || height == 0 {
            let error_msg = format!("Invalid canvas dimensions: {}x{}", width, height);
            console_log!("❌ {}", error_msg);
            return Err(JsValue::from_str(&error_msg));
        }

        // Set viewport
        gl.viewport(0, 0, width as i32, height as i32);

        console_log!("✅ Simple WebGL2 renderer created successfully");

        Ok(Self {
            canvas,
            gl,
            width,
            height,
            time_start: js_sys::Date::now(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            is_initialized: true,
            shader_content: None,
            shader_hash: 0,
        })
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        let current_time = (js_sys::Date::now() - self.time_start) / 1000.0;

        if self.shader_content.is_some() {
            // Render with shader-based colors
            self.render_with_shader_based_colors(current_time as f32);
        } else {
            // Default: Simple animated clear color based on time
            let r = (current_time.sin() * 0.5 + 0.5) as f32;
            let g = ((current_time + 2.0).sin() * 0.5 + 0.5) as f32;
            let b = ((current_time + 4.0).sin() * 0.5 + 0.5) as f32;

            // Add mouse influence
            let mouse_influence = (self.mouse_x / self.width as f32) * 0.3;

            self.gl.clear_color(r * (1.0 - mouse_influence), g, b * (1.0 + mouse_influence), 1.0);
            self.gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
        }

        Ok(())
    }

    fn render_with_shader_based_colors(&mut self, time: f32) {
        // Generate colors based on shader hash and time
        let hash_float = (self.shader_hash as f32) / (u32::MAX as f32);

        // Use shader hash to create unique color patterns
        let r = ((time * 0.5 + hash_float * 6.28).sin() * 0.5 + 0.5) as f32;
        let g = ((time * 0.7 + hash_float * 4.19).cos() * 0.5 + 0.5) as f32;
        let b = ((time * 0.3 + hash_float * 2.09).sin() * 0.5 + 0.5) as f32;

        // Add mouse influence
        let mouse_x_norm = self.mouse_x / self.width as f32;
        let mouse_y_norm = self.mouse_y / self.height as f32;

        let final_r = r * (0.7 + mouse_x_norm * 0.3);
        let final_g = g * (0.7 + mouse_y_norm * 0.3);
        let final_b = b * (0.7 + (mouse_x_norm + mouse_y_norm) * 0.15);

        console_log!("🎨 Rendering with shader-based colors: r={:.2}, g={:.2}, b={:.2} (hash: {})",
                     final_r, final_g, final_b, self.shader_hash);

        self.gl.clear_color(final_r, final_g, final_b, 1.0);
        self.gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
    }


    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.width = width;
            self.height = height;
            self.canvas.set_width(width);
            self.canvas.set_height(height);
            self.gl.viewport(0, 0, width as i32, height as i32);
            console_log!("✅ WebGL2 renderer resized to {}x{}", width, height);
        }
    }

    pub fn handle_mouse_input(&mut self, x: f64, y: f64) {
        self.mouse_x = x as f32;
        self.mouse_y = y as f32;
    }

    pub fn handle_ipc_message(&mut self, message: crate::ipc_protocol::IpcMessage) -> Result<(), JsValue> {
        console_log!("📨 Simple WebGL2 renderer handling IPC message");

        match message {
            crate::ipc_protocol::IpcMessage::ConfigUpdate(config_data) => {
                console_log!("🔄 Config update received (WebGL2 renderer)");
                if let Some(shader_config) = config_data.shader_config {
                    console_log!("🎨 Loading shader content: {}", shader_config.label);
                    self.load_shader_content(&shader_config.shader_source);
                    console_log!("✅ Shader content loaded successfully");
                }
            }
            crate::ipc_protocol::IpcMessage::UniformUpdate(_uniform_data) => {
                // Handle uniform updates if needed
                console_log!("📊 Uniform update received (WebGL2 renderer)");
            }
            _ => {
                console_log!("📨 Other IPC message received");
            }
        }

        Ok(())
    }

    fn load_shader_content(&mut self, shader_source: &str) {
        console_log!("📝 Loading shader content ({} chars)", shader_source.len());

        // Calculate a simple hash from shader content
        self.shader_hash = self.calculate_hash(shader_source);
        self.shader_content = Some(shader_source.to_string());

        console_log!("🔢 Shader hash: {} (from {} characters)", self.shader_hash, shader_source.len());
    }

    fn calculate_hash(&self, text: &str) -> u32 {
        let mut hash: u32 = 5381;
        for byte in text.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
        }
        hash
    }

    pub fn initialize_with_config(&mut self, _config_data: crate::ipc_protocol::ConfigData) -> Result<(), JsValue> {
        console_log!("⚙️ Simple WebGL2 renderer initializing with config");
        // For now, just mark as initialized
        self.is_initialized = true;
        console_log!("✅ Simple WebGL2 renderer initialized");
        Ok(())
    }

    pub fn get_stats(&self) -> (bool, usize, usize) {
        (self.is_initialized, 1, 0) // is_initialized, shader_configs, shader_cache
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

}