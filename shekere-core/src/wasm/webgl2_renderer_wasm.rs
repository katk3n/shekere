use wasm_bindgen::prelude::*;
use web_sys::{WebGlProgram, WebGlBuffer, WebGlTexture, WebGl2RenderingContext};
use std::collections::HashMap;
use crate::console_log;
use crate::wasm::webgl2_context_wasm::WebGl2ContextWasm;

pub struct WebGl2RendererWasm {
    context: WebGl2ContextWasm,
    current_program: Option<WebGlProgram>,
    quad_buffer: Option<WebGlBuffer>,
    textures: HashMap<String, WebGlTexture>,
    time_start: f64,
    mouse_x: f32,
    mouse_y: f32,
    is_initialized: bool,
}

impl WebGl2RendererWasm {
    pub fn new(context: WebGl2ContextWasm) -> Result<Self, JsValue> {
        console_log!("Creating WebGL2 renderer...");

        let mut renderer = Self {
            context,
            current_program: None,
            quad_buffer: None,
            textures: HashMap::new(),
            time_start: js_sys::Date::now(),
            mouse_x: 0.0,
            mouse_y: 0.0,
            is_initialized: false,
        };

        // Setup fullscreen quad
        renderer.setup_quad()?;

        console_log!("✅ WebGL2 renderer created successfully");
        Ok(renderer)
    }

    fn setup_quad(&mut self) -> Result<(), JsValue> {
        console_log!("Setting up fullscreen quad...");
        self.quad_buffer = Some(self.context.setup_fullscreen_quad()?);
        console_log!("✅ Fullscreen quad setup complete");
        Ok(())
    }

    pub fn load_shader(&mut self, fragment_shader_source: &str) -> Result<(), JsValue> {
        console_log!("Loading shader...");

        let gl = &self.context.gl;

        // Convert WGSL to GLSL
        let glsl_fragment = self.context.convert_wgsl_to_glsl(fragment_shader_source);
        console_log!("Converted WGSL to GLSL");

        // Create vertex shader
        let vertex_shader = self.context.create_shader(
            WebGl2RenderingContext::VERTEX_SHADER,
            WebGl2ContextWasm::get_default_vertex_shader(),
        )?;

        // Create fragment shader
        let fragment_shader = self.context.create_shader(
            WebGl2RenderingContext::FRAGMENT_SHADER,
            &glsl_fragment,
        )?;

        // Create program
        let program = self.context.create_program(&vertex_shader, &fragment_shader)?;

        // Clean up old program
        if let Some(old_program) = &self.current_program {
            gl.delete_program(Some(old_program));
        }

        self.current_program = Some(program);

        // Clean up shaders (they're linked into the program now)
        gl.delete_shader(Some(&vertex_shader));
        gl.delete_shader(Some(&fragment_shader));

        console_log!("✅ Shader loaded successfully");
        Ok(())
    }

    pub fn render(&mut self) -> Result<(), JsValue> {
        let gl = &self.context.gl;

        if let Some(program) = &self.current_program {
            // Clear the canvas
            self.context.clear(0.0, 0.0, 0.0, 1.0);

            // Use the shader program
            gl.use_program(Some(program));

            // Set uniforms
            self.set_uniforms(program)?;

            // Bind vertex buffer
            if let Some(buffer) = &self.quad_buffer {
                gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(buffer));

                // Setup vertex attributes
                let position_location = gl.get_attrib_location(program, "a_position") as u32;
                gl.enable_vertex_attrib_array(position_location);
                gl.vertex_attrib_pointer_with_i32(
                    position_location,
                    2, // size (2 components per iteration)
                    WebGl2RenderingContext::FLOAT, // type
                    false, // normalize
                    0, // stride
                    0, // offset
                );

                // Draw the quad
                gl.draw_arrays(WebGl2RenderingContext::TRIANGLES, 0, 6);
            }
        }

        Ok(())
    }

    fn set_uniforms(&self, program: &WebGlProgram) -> Result<(), JsValue> {
        let gl = &self.context.gl;

        // Time uniform
        let current_time = (js_sys::Date::now() - self.time_start) / 1000.0;
        if let Some(location) = gl.get_uniform_location(program, "u_time") {
            gl.uniform1f(Some(&location), current_time as f32);
        }

        // Resolution uniform
        if let Some(location) = gl.get_uniform_location(program, "u_resolution") {
            gl.uniform2f(Some(&location), self.context.width as f32, self.context.height as f32);
        }

        // Mouse uniform
        if let Some(location) = gl.get_uniform_location(program, "u_mouse") {
            gl.uniform2f(Some(&location), self.mouse_x, self.mouse_y);
        }

        // Texture uniforms
        for i in 0..4 {
            let uniform_name = format!("u_texture{}", i);
            if let Some(location) = gl.get_uniform_location(program, &uniform_name) {
                gl.uniform1i(Some(&location), i);
            }
        }

        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        console_log!("Resizing WebGL2 renderer to {}x{}", width, height);
        self.context.resize(width, height);
    }

    pub fn handle_mouse_input(&mut self, x: f64, y: f64) {
        self.mouse_x = x as f32;
        self.mouse_y = y as f32;
    }

    pub fn handle_ipc_message(&mut self, message: crate::ipc_protocol::IpcMessage) -> Result<(), JsValue> {
        console_log!("📨 WebGL2 renderer handling IPC message: {:?}", message.message_type);

        match message.message_type.as_str() {
            "ConfigUpdate" => {
                if let Some(shader_config) = message.data.shader_config {
                    console_log!("🔄 Loading shader from config update");
                    self.load_shader(&shader_config.shader_source)?;
                    self.is_initialized = true;
                    console_log!("✅ WebGL2 renderer initialized with shader");
                }
            }
            "UniformUpdate" => {
                // Handle uniform updates if needed
                console_log!("📊 Received uniform update (WebGL2 renderer)");
            }
            "HotReload" => {
                console_log!("🔥 Hot reload in WebGL2 renderer");
                if let Some(config) = message.data.hot_reload.and_then(|hr| hr.new_config) {
                    if let Some(shader_config) = config.shader_config {
                        self.load_shader(&shader_config.shader_source)?;
                        console_log!("✅ Hot reload completed");
                    }
                }
            }
            _ => {
                console_log!("⚠️ Unknown IPC message type: {}", message.message_type);
            }
        }

        Ok(())
    }

    pub fn initialize_with_config(&mut self, config_data: crate::ipc_protocol::ConfigData) -> Result<(), JsValue> {
        console_log!("⚙️ WebGL2 renderer initializing with config");

        if let Some(shader_config) = config_data.shader_config {
            self.load_shader(&shader_config.shader_source)?;
            self.is_initialized = true;
            console_log!("✅ WebGL2 renderer initialized with config");
        }

        Ok(())
    }

    pub fn get_stats(&self) -> (bool, usize, usize) {
        (
            self.is_initialized,
            if self.current_program.is_some() { 1 } else { 0 },
            self.textures.len(),
        )
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}