use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Config {
    pub window: WindowConfig,
    pub pipeline: Vec<ShaderConfig>,
    pub osc: Option<OscConfig>,
    pub spectrum: Option<SpectrumConfig>,
    pub midi: Option<MidiConfig>,
    pub hot_reload: Option<HotReloadConfig>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Copy)]
pub struct HotReloadConfig {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct ShaderConfig {
    pub shader_type: String,
    pub label: String,
    pub entry_point: String,
    pub file: String,
    pub ping_pong: Option<bool>,
    pub persistent: Option<bool>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct OscSoundConfig {
    pub name: String,
    pub id: i32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct OscConfig {
    pub port: u32,
    pub addr_pattern: String,
    pub sound: Vec<OscSoundConfig>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct SpectrumConfig {
    pub min_frequency: f32,
    pub max_frequency: f32,
    pub sampling_rate: u32,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct MidiConfig {
    pub enabled: bool,
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

            // Validate ping_pong and persistent flags
            if shader.ping_pong.unwrap_or(false) && shader.persistent.unwrap_or(false) {
                return Err(format!(
                    "Pipeline[{}]: ping_pong and persistent cannot both be true",
                    i
                ));
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
        assert_eq!(config.midi, None);
        assert_eq!(config.hot_reload, None);
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
                ping_pong: None,
                persistent: None,
            }],
            osc: None,
            spectrum: None,
            midi: None,
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
                ping_pong: None,
                persistent: None,
            }],
            osc: None,
            spectrum: None,
            midi: None,
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
            midi: None,
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
                ping_pong: None,
                persistent: None,
            }],
            osc: None,
            spectrum: Some(SpectrumConfig {
                min_frequency: 1000.0,
                max_frequency: 500.0,
                sampling_rate: 44100,
            }),
            midi: None,
            hot_reload: None,
        };

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_from_toml_with_hot_reload_enabled() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "test"
entry_point = "fs_main"
file = "test.wgsl"

[hot_reload]
enabled = true
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert_eq!(config.hot_reload.is_some(), true);
        assert_eq!(config.hot_reload.unwrap().enabled, true);
    }

    #[test]
    fn test_config_from_toml_with_hot_reload_disabled() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "test"
entry_point = "fs_main"
file = "test.wgsl"

[hot_reload]
enabled = false
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert_eq!(config.hot_reload.is_some(), true);
        assert_eq!(config.hot_reload.unwrap().enabled, false);
    }

    #[test]
    fn test_config_from_toml_without_hot_reload() {
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
        assert_eq!(config.hot_reload, None);
    }

    #[test]
    fn test_hot_reload_config_equality() {
        let config1 = HotReloadConfig { enabled: true };
        let config2 = HotReloadConfig { enabled: true };
        let config3 = HotReloadConfig { enabled: false };

        assert_eq!(config1, config2);
        assert_ne!(config1, config3);
    }

    #[test]
    fn test_config_from_toml_with_midi() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "midi"
entry_point = "fs_main"
file = "midi.wgsl"

[midi]
enabled = true
"#;

        let config = Config::from_toml(toml_str).unwrap();
        let midi = config.midi.unwrap();
        assert_eq!(midi.enabled, true);
    }

    #[test]
    fn test_config_with_all_features_including_hot_reload() {
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

[spectrum]
min_frequency = 20.0
max_frequency = 20000.0
sampling_rate = 44100

[midi]
enabled = true

[hot_reload]
enabled = true
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert!(config.validate().is_ok());

        // Check all components are present
        assert!(config.osc.is_some());
        assert!(config.spectrum.is_some());
        assert!(config.midi.is_some());
        assert!(config.hot_reload.is_some());
        assert_eq!(config.hot_reload.unwrap().enabled, true);
        assert_eq!(config.midi.unwrap().enabled, true);
    }

    #[test]
    fn test_shader_config_with_ping_pong_should_parse_after_implementation() {
        // TDD Red phase: This test should fail because ping_pong field doesn't exist yet
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Game of Life"
entry_point = "fs_main"
file = "life.wgsl"
ping_pong = true
"#;

        let result = Config::from_toml(toml_str);
        assert!(result.is_ok()); // Should parse successfully but ping_pong field is ignored
        let config = result.unwrap();

        // This should fail because ping_pong field doesn't exist yet
        let shader_config = &config.pipeline[0];
        // We can't access ping_pong field yet because it doesn't exist
        assert_eq!(shader_config.shader_type, "fragment");
        assert_eq!(shader_config.label, "Game of Life");
    }

    #[test]
    fn test_shader_config_with_persistent_should_parse_after_implementation() {
        // TDD Red phase: This test should fail because persistent field doesn't exist yet
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Trail Effect"
entry_point = "fs_main"
file = "trail.wgsl"
persistent = true
"#;

        let result = Config::from_toml(toml_str);
        assert!(result.is_ok()); // Should parse successfully but persistent field is ignored
        let config = result.unwrap();

        // This should fail because persistent field doesn't exist yet
        let shader_config = &config.pipeline[0];
        // We can't access persistent field yet because it doesn't exist
        assert_eq!(shader_config.shader_type, "fragment");
        assert_eq!(shader_config.label, "Trail Effect");
    }

    #[test]
    fn test_multi_pass_shader_config_should_fail() {
        // TDD Red phase: This test should fail because ping_pong field doesn't exist yet
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Main Render"
entry_point = "fs_main"
file = "main.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Blur Effect"
entry_point = "fs_main"
file = "blur.wgsl"
"#;

        let result = Config::from_toml(toml_str);
        assert!(result.is_ok()); // Multi-pass without special flags should work
        let config = result.unwrap();
        assert_eq!(config.pipeline.len(), 2);
    }

    #[test]
    fn test_shader_config_needs_ping_pong_field() {
        // This test demonstrates the need for ping_pong field access
        let config = ShaderConfig {
            shader_type: "fragment".to_string(),
            label: "Game of Life".to_string(),
            entry_point: "fs_main".to_string(),
            file: "life.wgsl".to_string(),
            ping_pong: None,
            persistent: None,
        };

        // Now we can access ping_pong field
        assert_eq!(config.ping_pong.unwrap_or(false), false);
        assert_eq!(config.shader_type, "fragment");
    }

    #[test]
    fn test_shader_config_needs_persistent_field() {
        // This test demonstrates the need for persistent field access
        let config = ShaderConfig {
            shader_type: "fragment".to_string(),
            label: "Trail Effect".to_string(),
            entry_point: "fs_main".to_string(),
            file: "trail.wgsl".to_string(),
            ping_pong: None,
            persistent: None,
        };

        // Now we can access persistent field
        assert_eq!(config.persistent.unwrap_or(false), false);
        assert_eq!(config.shader_type, "fragment");
    }

    #[test]
    fn test_shader_config_with_ping_pong_true() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Game of Life"
entry_point = "fs_main"
file = "life.wgsl"
ping_pong = true
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert_eq!(config.pipeline.len(), 1);
        assert_eq!(config.pipeline[0].ping_pong, Some(true));
        assert_eq!(config.pipeline[0].persistent, None);
        assert_eq!(config.pipeline[0].label, "Game of Life");
    }

    #[test]
    fn test_shader_config_with_persistent_true() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Trail Effect"
entry_point = "fs_main"
file = "trail.wgsl"
persistent = true
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert_eq!(config.pipeline.len(), 1);
        assert_eq!(config.pipeline[0].persistent, Some(true));
        assert_eq!(config.pipeline[0].ping_pong, None);
        assert_eq!(config.pipeline[0].label, "Trail Effect");
    }

    #[test]
    fn test_shader_config_with_both_ping_pong_and_persistent() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Complex Effect"
entry_point = "fs_main"
file = "complex.wgsl"
ping_pong = true
persistent = false
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert_eq!(config.pipeline.len(), 1);
        assert_eq!(config.pipeline[0].ping_pong, Some(true));
        assert_eq!(config.pipeline[0].persistent, Some(false));
        assert_eq!(config.pipeline[0].label, "Complex Effect");
    }

    #[test]
    fn test_shader_config_multi_pass_with_mixed_flags() {
        let toml_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Main Render"
entry_point = "fs_main"
file = "main.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Blur Effect"
entry_point = "fs_main"
file = "blur.wgsl"
ping_pong = true

[[pipeline]]
shader_type = "fragment"
label = "Final Composite"
entry_point = "fs_main"
file = "composite.wgsl"
persistent = true
"#;

        let config = Config::from_toml(toml_str).unwrap();
        assert_eq!(config.pipeline.len(), 3);

        // First pass - no special flags
        assert_eq!(config.pipeline[0].ping_pong, None);
        assert_eq!(config.pipeline[0].persistent, None);
        assert_eq!(config.pipeline[0].label, "Main Render");

        // Second pass - ping_pong enabled
        assert_eq!(config.pipeline[1].ping_pong, Some(true));
        assert_eq!(config.pipeline[1].persistent, None);
        assert_eq!(config.pipeline[1].label, "Blur Effect");

        // Third pass - persistent enabled
        assert_eq!(config.pipeline[2].ping_pong, None);
        assert_eq!(config.pipeline[2].persistent, Some(true));
        assert_eq!(config.pipeline[2].label, "Final Composite");
    }

    #[test]
    fn test_shader_config_validation_ping_pong_and_persistent_conflict() {
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
                ping_pong: Some(true),
                persistent: Some(true),
            }],
            osc: None,
            spectrum: None,
            midi: None,
            hot_reload: None,
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("ping_pong and persistent cannot both be true")
        );
    }
}
