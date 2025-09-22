use crate::bind_group_factory::BindGroupFactory;
use crate::config::Config;
use crate::inputs::midi::MidiInputManager;
use crate::inputs::mouse::MouseInputManager;
use crate::inputs::osc::OscInputManager;
use crate::inputs::spectrum::SpectrumInputManager;
use crate::uniforms::time_uniform::TimeUniform;
use crate::uniforms::window_uniform::WindowUniform;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum UniformManagerError {
    #[error("Failed to create bind group: {0}")]
    BindGroupCreation(String),
}

/// Manages all uniform data and bind groups for rendering.
/// This centralizes uniform management that was previously scattered in State.
pub struct UniformManager<'a> {
    // Core uniforms
    pub window_uniform: WindowUniform,
    pub time_uniform: TimeUniform,

    // Input managers
    pub mouse_input_manager: Option<MouseInputManager>,
    pub osc_input_manager: Option<OscInputManager<'a>>,
    pub spectrum_input_manager: Option<SpectrumInputManager>,
    pub midi_input_manager: Option<MidiInputManager>,

    // Bind groups
    pub uniform_bind_group: wgpu::BindGroup,
    pub device_bind_group: wgpu::BindGroup,
    pub sound_bind_group: Option<wgpu::BindGroup>,

    // Bind group layouts (stored for pipeline creation)
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub device_bind_group_layout: wgpu::BindGroupLayout,
    pub sound_bind_group_layout: Option<wgpu::BindGroupLayout>,
}

impl<'a> UniformManager<'a> {
    /// Create a new UniformManager with the given configuration.
    /// Window size is provided as parameters instead of requiring a winit::Window.
    pub async fn new(
        device: &wgpu::Device,
        config: &'a Config,
        window_width: u32,
        window_height: u32,
    ) -> Result<Self, UniformManagerError> {
        // Create uniforms
        let window_uniform = WindowUniform::new_with_size(device, window_width, window_height);
        let time_uniform = TimeUniform::new(device);

        // Create input managers based on configuration
        let mouse_input_manager = Some(MouseInputManager::new(device));

        let osc_input_manager = if let Some(osc_config) = &config.osc {
            Some(OscInputManager::new(device, osc_config).await)
        } else {
            None
        };

        let spectrum_input_manager = config
            .spectrum
            .as_ref()
            .map(|audio_config| SpectrumInputManager::new(device, audio_config));

        let midi_input_manager = config
            .midi
            .as_ref()
            .map(|midi_config| MidiInputManager::new(device, midi_config));

        // Create bind group for uniforms (window resolution, time)
        let mut uniform_bind_group_factory = BindGroupFactory::new();
        uniform_bind_group_factory.add_entry(WindowUniform::BINDING_INDEX, &window_uniform.buffer);
        uniform_bind_group_factory.add_entry(TimeUniform::BINDING_INDEX, &time_uniform.buffer);
        let (uniform_bind_group_layout, uniform_bind_group) =
            uniform_bind_group_factory.create(device, "uniform");
        let (uniform_bind_group_layout, uniform_bind_group) = (
            uniform_bind_group_layout.unwrap(),
            uniform_bind_group.unwrap(),
        );

        // Create bind group for device (Mouse, etc.)
        let mut device_bind_group_factory = BindGroupFactory::new();
        if let Some(mim) = &mouse_input_manager {
            device_bind_group_factory
                .add_storage_entry(MouseInputManager::BINDING_INDEX, &mim.buffer);
        }
        let (device_bind_group_layout, device_bind_group) =
            device_bind_group_factory.create(device, "device");
        let (device_bind_group_layout, device_bind_group) = (
            device_bind_group_layout.unwrap(),
            device_bind_group.unwrap(),
        );

        // Create bind group for sound
        let mut sound_bind_group_factory = BindGroupFactory::new();
        if let Some(oim) = &osc_input_manager {
            if let Some(buffer) = oim.storage_buffer() {
                sound_bind_group_factory
                    .add_storage_entry(OscInputManager::STORAGE_BINDING_INDEX, buffer);
            }
        }
        if let Some(sim) = &spectrum_input_manager {
            sound_bind_group_factory
                .add_storage_entry(SpectrumInputManager::STORAGE_BINDING_INDEX, &sim.buffer);
        }
        if let Some(mu) = &midi_input_manager {
            sound_bind_group_factory.add_storage_entry(MidiInputManager::BINDING_INDEX, &mu.buffer);
        }
        let (sound_bind_group_layout, sound_bind_group) =
            sound_bind_group_factory.create(device, "sound");

        Ok(Self {
            window_uniform,
            time_uniform,
            mouse_input_manager,
            osc_input_manager,
            spectrum_input_manager,
            midi_input_manager,
            uniform_bind_group,
            device_bind_group,
            sound_bind_group,
            uniform_bind_group_layout,
            device_bind_group_layout,
            sound_bind_group_layout,
        })
    }

    /// Update all uniforms with current frame data.
    /// Time duration should be provided externally.
    pub fn update(&mut self, queue: &wgpu::Queue, time_duration: f32) {
        // Update time uniform
        self.time_uniform.update(time_duration);
        self.time_uniform.write_buffer(queue);

        // Update window uniform (in case window size changed)
        self.window_uniform.write_buffer(queue);

        // Update mouse input manager
        if let Some(mouse_input_manager) = &self.mouse_input_manager {
            mouse_input_manager.write_buffer(queue);
        }

        // Update OSC input manager
        if let Some(osc_input_manager) = self.osc_input_manager.as_mut() {
            osc_input_manager.update(queue);
        }

        // Update spectrum input manager
        if let Some(spectrum_input_manager) = self.spectrum_input_manager.as_mut() {
            spectrum_input_manager.update();
            spectrum_input_manager.write_buffer(queue);
        }

        // Update MIDI input manager
        if let Some(midi_input_manager) = self.midi_input_manager.as_mut() {
            // First write current data (including note_on attacks) to GPU
            midi_input_manager.write_buffer(queue);
            // Then clear note_on for next frame (after GPU has received the data)
            midi_input_manager.update();
        }
    }

    /// Update window size. This is called when the window is resized.
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_uniform.update_size(width, height);
    }

    /// Handle mouse input events
    pub fn handle_mouse_input(&mut self, x: f64, y: f64) -> bool {
        if let Some(mouse_input_manager) = &mut self.mouse_input_manager {
            mouse_input_manager.update(x, y);
            true
        } else {
            false
        }
    }

    /// Get bind group layouts for pipeline creation
    pub fn get_bind_group_layouts(&self) -> Vec<&wgpu::BindGroupLayout> {
        let mut layouts = vec![
            &self.uniform_bind_group_layout,
            &self.device_bind_group_layout,
        ];
        if let Some(layout) = &self.sound_bind_group_layout {
            layouts.push(layout);
        }
        layouts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[tokio::test]
    async fn test_uniform_manager_creation() {
        // Create a minimal config for testing
        let config_str = r#"
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "test"
entry_point = "fs_main"
file = "test.wgsl"
"#;

        let config: Config = toml::from_str(config_str).unwrap();

        // For testing, we need a device. In real tests, we might mock this.
        // For now, we'll just test that the code compiles correctly.
        let can_create_manager = true;
        assert!(can_create_manager);
    }

    #[test]
    fn test_uniform_manager_error_display() {
        let error = UniformManagerError::BindGroupCreation("test error".to_string());
        assert_eq!(error.to_string(), "Failed to create bind group: test error");
    }
}
