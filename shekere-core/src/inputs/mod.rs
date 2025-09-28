#[cfg(not(target_arch = "wasm32"))]
pub mod midi;
pub mod mouse;
#[cfg(not(target_arch = "wasm32"))]
pub mod osc;
#[cfg(not(target_arch = "wasm32"))]
pub mod spectrum;
