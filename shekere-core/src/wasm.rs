// Re-export everything from the wasm module
mod webgpu_context_wasm;
mod wasm_renderer;
mod wasm_multi_pass_pipeline;
mod wasm_texture_manager;
mod shader_loader_wasm;
mod simple_webgl2_renderer;

pub use webgpu_context_wasm::WebGpuContextWasm;
pub use wasm_renderer::WasmRenderer;
pub use wasm_multi_pass_pipeline::WasmMultiPassPipeline;
pub use wasm_texture_manager::{WasmTextureManager, WasmTextureType};
pub use shader_loader_wasm::WasmShaderLoader;
pub use simple_webgl2_renderer::SimpleWebGl2Renderer;

use wasm_bindgen::prelude::*;

// Initialize panic hook
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

// Console logging utilities
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (crate::wasm::log(&format_args!($($t)*).to_string()))
}

// Test function for initial verification
#[wasm_bindgen]
pub fn greet() -> String {
    "Hello from shekere-core WASM!".to_string()
}

// Renderer types
pub enum RendererType {
    WebGpu(WasmRenderer),
    WebGl2(SimpleWebGl2Renderer),
}

// Main WASM interface with IPC support
#[wasm_bindgen]
pub struct WasmShekereCore {
    renderer: Option<RendererType>,
    is_initialized: bool,
}

#[wasm_bindgen]
impl WasmShekereCore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmShekereCore {
        console_log!("🚀 Creating WasmShekereCore with IPC support");
        WasmShekereCore {
            renderer: None,
            is_initialized: false,
        }
    }

    #[wasm_bindgen]
    pub async fn init_with_canvas(&mut self, canvas: web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
        console_log!("🎨 Initializing WASM with canvas...");
        console_log!("🔍 Canvas tag name: {}", canvas.tag_name());
        console_log!("🔍 Canvas width: {}, height: {}", canvas.width(), canvas.height());

        // Try WebGPU first
        console_log!("🔧 Attempting WebGPU initialization...");
        match WebGpuContextWasm::new(&canvas).await {
            Ok(webgpu_context) => {
                console_log!("✅ WebGPU context created successfully");

                match WasmRenderer::new(webgpu_context) {
                    Ok(renderer) => {
                        console_log!("✅ WebGPU renderer created successfully");
                        self.renderer = Some(RendererType::WebGpu(renderer));
                        self.is_initialized = true;
                        console_log!("✅ WASM ShekereCore fully initialized with WebGPU");
                        return Ok(());
                    },
                    Err(e) => {
                        console_log!("❌ WebGPU renderer creation failed: {:?}", e);
                    }
                }
            },
            Err(e) => {
                console_log!("❌ WebGPU context creation failed: {:?}", e);
            }
        }

        // Fallback to WebGL2
        console_log!("🔄 Falling back to WebGL2...");

        match SimpleWebGl2Renderer::new(canvas) {
            Ok(renderer) => {
                console_log!("✅ Simple WebGL2 renderer created successfully");
                self.renderer = Some(RendererType::WebGl2(renderer));
                self.is_initialized = true;
                console_log!("✅ WASM ShekereCore fully initialized with WebGL2");
                return Ok(());
            },
            Err(e) => {
                console_log!("❌ WebGL2 renderer creation failed: {:?}", e);
                return Err(JsValue::from_str("Neither WebGPU nor WebGL2 are available in this environment"));
            }
        }
    }

    #[wasm_bindgen]
    pub fn render(&mut self) -> Result<(), JsValue> {
        if let Some(renderer) = &mut self.renderer {
            match renderer {
                RendererType::WebGpu(r) => r.render()?,
                RendererType::WebGl2(r) => r.render()?,
            }
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(renderer) = &mut self.renderer {
            match renderer {
                RendererType::WebGpu(r) => r.resize(width, height),
                RendererType::WebGl2(r) => r.resize(width, height),
            }
        }
    }

    /// Handle IPC message from Tauri backend (JSON string)
    #[wasm_bindgen]
    pub fn handle_ipc_message(&mut self, message_json: &str) -> Result<(), JsValue> {
        console_log!("📨 Received IPC message: {}", message_json);

        // Parse JSON message
        let message: crate::ipc_protocol::IpcMessage = serde_json::from_str(message_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse IPC message: {}", e)))?;

        // Forward to renderer if available
        if let Some(renderer) = &mut self.renderer {
            match renderer {
                RendererType::WebGpu(r) => r.handle_ipc_message(message)?,
                RendererType::WebGl2(r) => r.handle_ipc_message(message)?,
            }
        } else {
            console_log!("⚠️ Received IPC message but renderer not initialized");
        }

        Ok(())
    }

    /// Initialize renderer with configuration (alternative to IPC)
    #[wasm_bindgen]
    pub fn initialize_with_config(&mut self, config_json: &str) -> Result<(), JsValue> {
        console_log!("⚙️ Initializing with config: {}", config_json);

        let config_data: crate::ipc_protocol::ConfigData = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse config: {}", e)))?;

        if let Some(renderer) = &mut self.renderer {
            match renderer {
                RendererType::WebGpu(r) => r.initialize_with_config(config_data)?,
                RendererType::WebGl2(r) => r.initialize_with_config(config_data)?,
            }
            console_log!("✅ Renderer initialized with config");
        } else {
            return Err(JsValue::from_str("Canvas not initialized. Call init_with_canvas first."));
        }

        Ok(())
    }

    /// Handle mouse input
    #[wasm_bindgen]
    pub fn handle_mouse_input(&mut self, x: f64, y: f64) {
        if let Some(renderer) = &mut self.renderer {
            match renderer {
                RendererType::WebGpu(r) => r.handle_mouse_input(x, y),
                RendererType::WebGl2(r) => r.handle_mouse_input(x, y),
            }
        }
    }

    /// Get renderer statistics as JSON string
    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        if let Some(renderer) = &self.renderer {
            let (is_initialized, shader_configs, shader_cache) = match renderer {
                RendererType::WebGpu(r) => r.get_stats(),
                RendererType::WebGl2(r) => r.get_stats(),
            };
            let renderer_type = match renderer {
                RendererType::WebGpu(_) => "WebGPU",
                RendererType::WebGl2(_) => "WebGL2",
            };
            let stats = serde_json::json!({
                "core_initialized": self.is_initialized,
                "renderer_initialized": is_initialized,
                "renderer_type": renderer_type,
                "shader_configs": shader_configs,
                "shader_cache": shader_cache
            });
            stats.to_string()
        } else {
            serde_json::json!({
                "core_initialized": self.is_initialized,
                "renderer_initialized": false,
                "renderer_type": "None",
                "shader_configs": 0,
                "shader_cache": 0
            }).to_string()
        }
    }

    /// Check if the core is fully initialized
    #[wasm_bindgen]
    pub fn is_initialized(&self) -> bool {
        let has_renderer = self.renderer.is_some();
        let result = self.is_initialized && has_renderer;

        if !result {
            console_log!("🔍 is_initialized check: flag={}, renderer={}, result={}",
                self.is_initialized, has_renderer, result);
        }

        result
    }
}