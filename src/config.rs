use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
    pub pipeline: Vec<ShaderConfig>,
    pub osc: Option<OscConfig>,
    pub spectrum: Option<SpectrumConfig>,
}

#[derive(Debug, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
pub struct ShaderConfig {
    pub shader_type: String,
    pub label: String,
    pub entry_point: String,
    pub file: String,
}

#[derive(Debug, Deserialize)]
pub struct OscSoundConfig {
    pub name: String,
    pub id: i32,
}

#[derive(Debug, Deserialize)]
pub struct OscConfig {
    pub port: u32,
    pub addr_pattern: String,
    pub sound: Vec<OscSoundConfig>,
}

#[derive(Debug, Deserialize)]
pub struct SpectrumConfig {
    pub min_frequency: f32,
    pub max_frequency: f32,
    pub sampling_rate: u32,
}
