use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    pub window: WindowConfig,
    pub pipeline: Vec<ShaderConfig>,
    pub osc: Option<OscConfig>,
    pub spectrum: Option<SpectrumConfig>,
    pub hot_reload: Option<HotReloadConfig>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct HotReloadConfig {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ShaderConfig {
    pub shader_type: String,
    pub label: String,
    pub entry_point: String,
    pub file: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct OscSoundConfig {
    pub name: String,
    pub id: i32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct OscConfig {
    pub port: u32,
    pub addr_pattern: String,
    pub sound: Vec<OscSoundConfig>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct SpectrumConfig {
    pub min_frequency: f32,
    pub max_frequency: f32,
    pub sampling_rate: u32,
}

impl Config {
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.window.width == 0 || self.window.height == 0 {
            return Err("Window dimensions must be greater than 0".to_string());
        }

        if self.pipeline.is_empty() {
            return Err("At least one shader pipeline must be configured".to_string());
        }

        for (i, shader) in self.pipeline.iter().enumerate() {
            if shader.shader_type.is_empty() {
                return Err(format!("Pipeline[{}]: shader_type cannot be empty", i));
            }
            if shader.file.is_empty() {
                return Err(format!("Pipeline[{}]: file cannot be empty", i));
            }
        }

        if let Some(spectrum) = &self.spectrum {
            if spectrum.min_frequency >= spectrum.max_frequency {
                return Err("Spectrum min_frequency must be less than max_frequency".to_string());
            }
            if spectrum.sampling_rate == 0 {
                return Err("Spectrum sampling_rate must be greater than 0".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_toml_basic() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "test"
entry_point = "fs_main"
file = "test.wgsl"
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert_eq!(config.window.width, 800);
        assert_eq!(config.window.height, 600);
        assert_eq!(config.pipeline.len(), 1);
        assert_eq!(config.pipeline[0].shader_type, "fragment");
        assert_eq!(config.osc, None);
        assert_eq!(config.spectrum, None);
    }

    #[test]
    fn test_config_from_toml_with_osc() {
        let toml_str = r#"
[window]
width = 1280
height = 720

[[pipeline]]
shader_type = "fragment"
label = "main"
entry_point = "fs_main"
file = "shader.wgsl"

[osc]
port = 57120
addr_pattern = "/play"

[[osc.sound]]
name = "kick"
id = 0

[[osc.sound]]
name = "snare"
id = 1
"#;

        let config = Config::from_toml(toml_str).unwrap();
        let osc = config.osc.unwrap();
        assert_eq!(osc.port, 57120);
        assert_eq!(osc.addr_pattern, "/play");
        assert_eq!(osc.sound.len(), 2);
        assert_eq!(osc.sound[0].name, "kick");
        assert_eq!(osc.sound[0].id, 0);
    }

    #[test]
    fn test_config_from_toml_with_spectrum() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "spectrum"
entry_point = "fs_main"
file = "spectrum.wgsl"

[spectrum]
min_frequency = 20.0
max_frequency = 20000.0
sampling_rate = 44100
"#;

        let config = Config::from_toml(toml_str).unwrap();
        let spectrum = config.spectrum.unwrap();
        assert_eq!(spectrum.min_frequency, 20.0);
        assert_eq!(spectrum.max_frequency, 20000.0);
        assert_eq!(spectrum.sampling_rate, 44100);
    }

    #[test]
    fn test_config_validate_success() {
        let config = Config {
            window: WindowConfig {
                width: 800,
                height: 600,
            },
            pipeline: vec![ShaderConfig {
                shader_type: "fragment".to_string(),
                label: "test".to_string(),
                entry_point: "fs_main".to_string(),
                file: "test.wgsl".to_string(),
            }],
            osc: None,
            spectrum: None,
            hot_reload: None,
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_zero_dimensions() {
        let config = Config {
            window: WindowConfig {
                width: 0,
                height: 600,
            },
            pipeline: vec![ShaderConfig {
                shader_type: "fragment".to_string(),
                label: "test".to_string(),
                entry_point: "fs_main".to_string(),
                file: "test.wgsl".to_string(),
            }],
            osc: None,
            spectrum: None,
            hot_reload: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_empty_pipeline() {
        let config = Config {
            window: WindowConfig {
                width: 800,
                height: 600,
            },
            pipeline: vec![],
            osc: None,
            spectrum: None,
            hot_reload: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_invalid_spectrum() {
        let config = Config {
            window: WindowConfig {
                width: 800,
                height: 600,
            },
            pipeline: vec![ShaderConfig {
                shader_type: "fragment".to_string(),
                label: "test".to_string(),
                entry_point: "fs_main".to_string(),
                file: "test.wgsl".to_string(),
            }],
            osc: None,
            spectrum: Some(SpectrumConfig {
                min_frequency: 1000.0,
                max_frequency: 500.0,
                sampling_rate: 44100,
            }),
            hot_reload: None,
        };

        assert!(config.validate().is_err());
    }
}
