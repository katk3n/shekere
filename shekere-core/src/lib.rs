mod bind_group_factory;
pub mod config;
pub mod hot_reload;
pub mod ipc_protocol;
mod inputs;
pub mod pipeline;
pub mod render_constants;
pub mod renderer;
mod shader_preprocessor;
mod state;
pub mod texture_manager;
pub mod timer;
#[cfg(not(target_arch = "wasm32"))]
pub mod uniform_manager;
#[cfg(target_arch = "wasm32")]
pub mod uniform_manager_minimal;
#[cfg(target_arch = "wasm32")]
pub use uniform_manager_minimal as uniform_manager;
mod uniforms;
mod vertex;
pub mod webgpu_context;

pub use crate::config::Config;
pub use crate::config::ShaderConfig;
pub use crate::ipc_protocol::{IpcMessage, UniformData, ConfigData, ErrorData};
pub use crate::renderer::{Renderer, RendererError};
pub use crate::timer::Timer;
pub use crate::uniform_manager::{UniformManager, UniformManagerError};
pub use crate::webgpu_context::{WebGpuContext, WebGpuError};

// WASM-specific exports
#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;
