/// WASM-compatible Shader Loader for browser-based dynamic shader loading
/// Handles shader preprocessing, IPC loading, and hot reload functionality

use crate::config::ShaderConfig;
use crate::ipc_protocol::{ConfigData, HotReloadData, ShaderType as IpcShaderType};
use wasm_bindgen::prelude::*;
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Embedded common shader definitions (same as native common.wgsl)
const EMBEDDED_SHADER_DEFS: &str = r#"// Embedded common definitions for shekere shaders
// This file is automatically included at the beginning of every shader

// === UNIFORM STRUCTURES ===

struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
}

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
}

struct SpectrumShaderData {
    // frequency values of audio input (packed into vec4s for alignment)
    frequencies: array<vec4<f32>, 512>,
    // amplitude values of audio input (packed into vec4s for alignment)
    amplitudes: array<vec4<f32>, 512>,
    // the number of data points
    num_points: u32,
    // frequency of the data point with the max amplitude
    max_frequency: f32,
    // max amplitude of audio input
    max_amplitude: f32,
    _padding: u32,
}

struct SpectrumHistory {
    // 512 frames of spectrum history data
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<SpectrumShaderData, 512>,
}

struct OscShaderData {
    // OSC sound values (packed into vec4s for alignment)
    sounds: array<vec4<i32>, 4>,
    // OSC ttl values (packed into vec4s for alignment)
    ttls: array<vec4<f32>, 4>,
    // OSC note values (packed into vec4s for alignment)
    notes: array<vec4<f32>, 4>,
    // OSC gain values (packed into vec4s for alignment)
    gains: array<vec4<f32>, 4>,
}

struct OscHistory {
    // 512 frames of OSC history data
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<OscShaderData, 512>,
}

struct MidiShaderData {
    // note velocities (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    notes: array<vec4<f32>, 32>,
    // control change values (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    controls: array<vec4<f32>, 32>,
    // note on attack detection (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    note_on: array<vec4<f32>, 32>,
}

struct MidiHistory {
    // 512 frames of MIDI history data (768KB total)
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<MidiShaderData, 512>,
}

struct MouseShaderData {
    // mouse position (vec2 with vec4 alignment)
    position: vec2<f32>,
    _padding: vec2<f32>, // vec4 alignment for GPU efficiency
}

struct MouseHistory {
    // 512 frames of mouse history data (8KB total)
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<MouseShaderData, 512>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

// === UNIFORM BINDINGS ===

// Group 0: Always available uniforms
@group(0) @binding(0) var<uniform> Window: WindowUniform;
@group(0) @binding(1) var<uniform> Time: TimeUniform;

// Group 1: Device uniforms (conditional)
@group(1) @binding(0) var<storage, read> Mouse: MouseHistory;

// Group 2: Sound uniforms (conditional - only bind what you use)
@group(2) @binding(0) var<storage, read> Osc: OscHistory;
@group(2) @binding(1) var<storage, read> Spectrum: SpectrumHistory;
@group(2) @binding(2) var<storage, read> Midi: MidiHistory;

// Group 3: Multi-pass textures (conditional - only available in multi-pass shaders)
@group(3) @binding(0) var previous_pass: texture_2d<f32>;
@group(3) @binding(1) var texture_sampler: sampler;

// === UTILITY FUNCTIONS ===

// Coordinate system helpers
fn NormalizedCoords(position: vec2<f32>) -> vec2<f32> {
    let min_xy = min(Window.resolution.x, Window.resolution.y);
    return (position * 2.0 - Window.resolution) / min_xy;
}

fn MouseCoordsHistory(history: u32) -> vec2<f32> {
    if history >= 512u {
        return vec2<f32>(0.0, 0.0);
    }

    let mouse_data = Mouse.history_data[history];
    return NormalizedCoords(mouse_data.position);
}

fn MouseCoords() -> vec2<f32> {
    return MouseCoordsHistory(0u);
}

// Color space conversion
fn ToLinearRgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

fn ToSrgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 1.0 / 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

// Multi-pass texture helper functions
fn SamplePreviousPass(uv: vec2<f32>) -> vec4<f32> {
    // Fix Y-axis flipping for persistent textures
    let corrected_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return textureSample(previous_pass, texture_sampler, corrected_uv);
}

fn SamplePreviousPassOffset(uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32> {
    return textureSample(previous_pass, texture_sampler, uv + offset);
}
"#;

/// WASM Shader Loader for dynamic shader management
pub struct WasmShaderLoader {
    /// Cached shader sources (file_name -> preprocessed_source)
    shader_cache: HashMap<String, String>,
    /// Current shader configurations
    shader_configs: Vec<ShaderConfig>,
    /// Raw shader sources before preprocessing (for hot reload)
    raw_shader_sources: HashMap<String, String>,
    /// Version tracking for hot reload
    shader_versions: HashMap<String, u64>,
}

impl WasmShaderLoader {
    /// Create a new WASM shader loader
    pub fn new() -> Self {
        console_log!("📚 Creating WASM ShaderLoader");

        Self {
            shader_cache: HashMap::new(),
            shader_configs: Vec::new(),
            raw_shader_sources: HashMap::new(),
            shader_versions: HashMap::new(),
        }
    }

    /// Load initial configuration from IPC
    pub fn load_config(&mut self, config_data: ConfigData) -> Result<(), String> {
        console_log!("⚙️ Loading shader configuration via IPC");

        // Convert IPC shader config to internal format if provided
        self.shader_configs.clear();
        if let Some(shader_config) = config_data.shader_config {
            let internal_config = ShaderConfig {
                shader_type: match shader_config.shader_type {
                    IpcShaderType::Fragment => "fragment".to_string(),
                    IpcShaderType::Compute => "compute".to_string(),
                    IpcShaderType::Vertex => "vertex".to_string(),
                },
                label: shader_config.label.clone(),
                entry_point: shader_config.entry_point.clone(),
                file: format!("{}.wgsl", shader_config.label.to_lowercase().replace(" ", "_")),
                ping_pong: Some(false), // Default for now
                persistent: Some(false), // Default for now
            };
            self.shader_configs.push(internal_config);

            // Store raw shader source
            let file_name = format!("{}.wgsl", shader_config.label.to_lowercase().replace(" ", "_"));
            self.raw_shader_sources.insert(file_name.clone(), shader_config.shader_source);
            self.shader_versions.insert(file_name, 0);
        }

        // Handle hot reload if provided
        if let Some(hot_reload_data) = config_data.hot_reload {
            self.handle_hot_reload(hot_reload_data)?;
        }

        // Preprocess all shaders if we have any
        if !self.shader_configs.is_empty() {
            self.preprocess_all_shaders()?;
        }

        console_log!("✅ Loaded {} shader configurations", self.shader_configs.len());
        Ok(())
    }

    /// Handle hot reload update
    pub fn handle_hot_reload(&mut self, hot_reload_data: HotReloadData) -> Result<bool, String> {
        console_log!("🔥 Handling hot reload for: {}", hot_reload_data.file_path);

        let file_name = hot_reload_data.file_path;
        let new_version = self.shader_versions.get(&file_name).unwrap_or(&0) + 1;

        // Update version (content should be provided separately via ConfigUpdate)
        self.shader_versions.insert(file_name.clone(), new_version);

        // Find affected shader configs
        let affected_configs: Vec<usize> = self.shader_configs
            .iter()
            .enumerate()
            .filter(|(_, config)| config.file == file_name)
            .map(|(index, _)| index)
            .collect();

        if affected_configs.is_empty() {
            console_log!("⚠️ No shader configs found for hot reload file: {}", file_name);
            return Ok(false);
        }

        // Reprocess affected shaders
        for &config_index in &affected_configs {
            let config = &self.shader_configs[config_index];
            let enable_multipass = config.ping_pong.unwrap_or(false)
                || config.persistent.unwrap_or(false)
                || config_index > 0;

            let processed_source = self.preprocess_shader_source(&file_name, enable_multipass)?;
            self.shader_cache.insert(file_name.clone(), processed_source);

            console_log!("🔄 Reprocessed shader for config: {}", config.label);
        }

        console_log!("✅ Hot reload completed for {} affected shader configs", affected_configs.len());
        Ok(true)
    }

    /// Preprocess all cached shader sources
    fn preprocess_all_shaders(&mut self) -> Result<(), String> {
        for (file_name, _) in &self.raw_shader_sources.clone() {
            // Determine if multipass is needed for this shader
            let enable_multipass = self.shader_configs
                .iter()
                .enumerate()
                .any(|(index, config)| {
                    config.file == *file_name && (
                        config.ping_pong.unwrap_or(false) ||
                        config.persistent.unwrap_or(false) ||
                        index > 0
                    )
                });

            let processed_source = self.preprocess_shader_source(file_name, enable_multipass)?;
            self.shader_cache.insert(file_name.clone(), processed_source);
        }

        console_log!("🔧 Preprocessed {} shaders", self.shader_cache.len());
        Ok(())
    }

    /// Preprocess a single shader source with embedded definitions
    fn preprocess_shader_source(&self, file_name: &str, enable_multipass: bool) -> Result<String, String> {
        let raw_source = self.raw_shader_sources
            .get(file_name)
            .ok_or_else(|| format!("Shader source not found: {}", file_name))?;

        // Process common.wgsl with conditional multipass bindings
        let processed_common = if enable_multipass {
            EMBEDDED_SHADER_DEFS.to_string()
        } else {
            // Remove Group 3 multipass bindings and functions for non-multipass shaders
            let lines: Vec<&str> = EMBEDDED_SHADER_DEFS.lines().collect();
            let mut filtered_lines = Vec::new();
            let mut skip_function = false;

            for line in lines {
                // Skip Group 3 bindings
                if line.contains("@group(3)")
                    || line.contains("var previous_pass:")
                    || line.contains("var texture_sampler:")
                {
                    continue;
                }

                // Skip SamplePreviousPass functions
                if line.contains("fn SamplePreviousPass") {
                    skip_function = true;
                    continue;
                }

                // End of function
                if skip_function && line.trim() == "}" {
                    skip_function = false;
                    continue;
                }

                // Skip lines inside function
                if skip_function {
                    continue;
                }

                filtered_lines.push(line);
            }

            filtered_lines.join("\n")
        };

        // Prepend processed embedded definitions to user shader
        let mut final_content = String::new();
        final_content.push_str(&processed_common);
        final_content.push_str("\n\n// === USER SHADER CODE ===\n\n");
        final_content.push_str(raw_source);

        console_log!("📝 Preprocessed shader: {} (multipass: {})", file_name, enable_multipass);
        Ok(final_content)
    }

    /// Get preprocessed shader source
    pub fn get_shader_source(&self, file_name: &str) -> Option<&String> {
        self.shader_cache.get(file_name)
    }

    /// Get all shader configurations
    pub fn get_shader_configs(&self) -> &[ShaderConfig] {
        &self.shader_configs
    }

    /// Get all preprocessed shader sources as HashMap (for pipeline creation)
    pub fn get_all_shader_sources(&self) -> HashMap<String, String> {
        self.shader_cache.clone()
    }

    /// Check if a shader has been loaded
    pub fn has_shader(&self, file_name: &str) -> bool {
        self.shader_cache.contains_key(file_name)
    }

    /// Get shader version (for tracking changes)
    pub fn get_shader_version(&self, file_name: &str) -> Option<u64> {
        self.shader_versions.get(file_name).copied()
    }

    /// Get statistics about loaded shaders
    pub fn get_stats(&self) -> (usize, usize, usize) {
        (
            self.shader_configs.len(),
            self.shader_cache.len(),
            self.raw_shader_sources.len()
        )
    }

    /// Clear all cached data
    pub fn clear(&mut self) {
        console_log!("🧹 Clearing shader loader cache");

        self.shader_cache.clear();
        self.shader_configs.clear();
        self.raw_shader_sources.clear();
        self.shader_versions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc_protocol::{ShaderConfigData, ShaderType as IpcShaderType};

    #[test]
    fn test_shader_loader_creation() {
        let loader = WasmShaderLoader::new();
        let (configs, cache, sources) = loader.get_stats();

        assert_eq!(configs, 0);
        assert_eq!(cache, 0);
        assert_eq!(sources, 0);
    }

    #[test]
    fn test_multipass_preprocessing() {
        let loader = WasmShaderLoader::new();

        // Test multipass enabled
        let multipass_result = loader.preprocess_shader_source("test.wgsl", true);
        assert!(multipass_result.is_err()); // Should fail because no raw source exists

        // Test embedded shader defs contain required bindings
        assert!(EMBEDDED_SHADER_DEFS.contains("@group(0)"));
        assert!(EMBEDDED_SHADER_DEFS.contains("@group(1)"));
        assert!(EMBEDDED_SHADER_DEFS.contains("@group(2)"));
        assert!(EMBEDDED_SHADER_DEFS.contains("@group(3)"));
        assert!(EMBEDDED_SHADER_DEFS.contains("SamplePreviousPass"));
    }

    #[test]
    fn test_shader_version_tracking() {
        let mut loader = WasmShaderLoader::new();

        // Initially no version
        assert_eq!(loader.get_shader_version("test.wgsl"), None);

        // Add a version
        loader.shader_versions.insert("test.wgsl".to_string(), 5);
        assert_eq!(loader.get_shader_version("test.wgsl"), Some(5));
    }

    #[test]
    fn test_embedded_shader_defs_completeness() {
        // Test that embedded shader defs contain essential structures and functions
        assert!(EMBEDDED_SHADER_DEFS.contains("struct WindowUniform"));
        assert!(EMBEDDED_SHADER_DEFS.contains("struct TimeUniform"));
        assert!(EMBEDDED_SHADER_DEFS.contains("fn MouseCoords"));
        assert!(EMBEDDED_SHADER_DEFS.contains("fn NormalizedCoords"));
        assert!(EMBEDDED_SHADER_DEFS.contains("fn ToLinearRgb"));
        assert!(EMBEDDED_SHADER_DEFS.contains("fn ToSrgb"));
    }
}