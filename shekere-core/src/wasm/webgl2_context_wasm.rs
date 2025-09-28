use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext};
use crate::console_log;

pub struct WebGl2ContextWasm {
    pub gl: WebGl2RenderingContext,
    pub canvas: HtmlCanvasElement,
    pub width: u32,
    pub height: u32,
}

impl WebGl2ContextWasm {
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        console_log!("Initializing WebGL2 context for WASM...");

        let gl = canvas
            .get_context("webgl2")?
            .ok_or("WebGL2 not supported")?
            .dyn_into::<WebGl2RenderingContext>()?;

        console_log!("✅ WebGL2 context created successfully");

        let width = canvas.width();
        let height = canvas.height();

        console_log!("Canvas dimensions: {}x{}", width, height);

        if width == 0 || height == 0 {
            let error_msg = format!("Invalid canvas dimensions: {}x{}", width, height);
            console_log!("❌ {}", error_msg);
            return Err(JsValue::from_str(&error_msg));
        }

        // Set viewport
        gl.viewport(0, 0, width as i32, height as i32);

        // Enable basic GL features
        gl.enable(WebGl2RenderingContext::BLEND);
        gl.blend_func(
            WebGl2RenderingContext::SRC_ALPHA,
            WebGl2RenderingContext::ONE_MINUS_SRC_ALPHA,
        );

        console_log!("✅ WebGL2 context initialized successfully");

        Ok(Self {
            gl,
            canvas: canvas.clone(),
            width,
            height,
        })
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            self.width = new_width;
            self.height = new_height;
            self.canvas.set_width(new_width);
            self.canvas.set_height(new_height);
            self.gl.viewport(0, 0, new_width as i32, new_height as i32);
        }
    }

    pub fn create_shader(&self, shader_type: u32, source: &str) -> Result<WebGlShader, JsValue> {
        let shader = self.gl
            .create_shader(shader_type)
            .ok_or("Unable to create shader object")?;

        self.gl.shader_source(&shader, source);
        self.gl.compile_shader(&shader);

        if self.gl
            .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            let info = self.gl
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| "Unknown error creating shader".to_string());
            self.gl.delete_shader(Some(&shader));
            Err(JsValue::from_str(&format!("Shader compilation error: {}", info)))
        }
    }

    pub fn create_program(&self, vertex_shader: &WebGlShader, fragment_shader: &WebGlShader) -> Result<WebGlProgram, JsValue> {
        let program = self.gl
            .create_program()
            .ok_or("Unable to create shader program")?;

        self.gl.attach_shader(&program, vertex_shader);
        self.gl.attach_shader(&program, fragment_shader);
        self.gl.link_program(&program);

        if self.gl
            .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(program)
        } else {
            let info = self.gl
                .get_program_info_log(&program)
                .unwrap_or_else(|| "Unknown error creating program".to_string());
            self.gl.delete_program(Some(&program));
            Err(JsValue::from_str(&format!("Program linking error: {}", info)))
        }
    }

    pub fn create_buffer(&self) -> Result<WebGlBuffer, JsValue> {
        self.gl
            .create_buffer()
            .ok_or("Failed to create buffer")
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn create_texture(&self) -> Result<WebGlTexture, JsValue> {
        self.gl
            .create_texture()
            .ok_or("Failed to create texture")
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn clear(&self, r: f32, g: f32, b: f32, a: f32) {
        self.gl.clear_color(r, g, b, a);
        self.gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
    }

    pub fn setup_fullscreen_quad(&self) -> Result<WebGlBuffer, JsValue> {
        let vertices: [f32; 12] = [
            -1.0, -1.0,
             1.0, -1.0,
            -1.0,  1.0,
            -1.0,  1.0,
             1.0, -1.0,
             1.0,  1.0,
        ];

        let buffer = self.create_buffer()?;
        self.gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

        unsafe {
            let positions_array_buf_view = js_sys::Float32Array::view(&vertices);
            self.gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &positions_array_buf_view,
                WebGl2RenderingContext::STATIC_DRAW,
            );
        }

        Ok(buffer)
    }

    pub fn get_default_vertex_shader() -> &'static str {
        r#"#version 300 es
        in vec2 a_position;
        out vec2 v_texCoord;

        void main() {
            gl_Position = vec4(a_position, 0.0, 1.0);
            v_texCoord = a_position * 0.5 + 0.5;
        }"#
    }

    pub fn convert_wgsl_to_glsl(&self, wgsl_source: &str) -> String {
        // Basic WGSL to GLSL conversion
        // This is a simplified converter - in a real implementation you'd want a proper parser
        let mut glsl = String::from("#version 300 es\nprecision highp float;\n");

        // Add common uniforms
        glsl.push_str("uniform float u_time;\n");
        glsl.push_str("uniform vec2 u_resolution;\n");
        glsl.push_str("uniform vec2 u_mouse;\n");
        glsl.push_str("uniform sampler2D u_texture0;\n");
        glsl.push_str("uniform sampler2D u_texture1;\n");
        glsl.push_str("uniform sampler2D u_texture2;\n");
        glsl.push_str("uniform sampler2D u_texture3;\n");
        glsl.push_str("in vec2 v_texCoord;\n");
        glsl.push_str("out vec4 fragColor;\n\n");

        // Basic WGSL to GLSL replacements
        let converted = wgsl_source
            .replace("@fragment", "")
            .replace("@vertex", "")
            .replace("fn fs_main", "void main")
            .replace("fn vs_main", "void main")
            .replace("-> @location(0) vec4<f32>", "")
            .replace("vec4<f32>", "vec4")
            .replace("vec3<f32>", "vec3")
            .replace("vec2<f32>", "vec2")
            .replace("f32", "float")
            .replace("return ", "fragColor = ")
            .replace("textureSample(", "texture(")
            .replace("u_time.value", "u_time")
            .replace("u_resolution.value", "u_resolution")
            .replace("u_mouse.value", "u_mouse");

        glsl.push_str(&converted);

        // Ensure we have a main function if not present
        if !glsl.contains("void main") {
            glsl.push_str("\nvoid main() {\n    fragColor = vec4(0.5, 0.8, 1.0, 1.0);\n}\n");
        }

        glsl
    }
}