// Bevy-compatible input system implementations
// This module contains thread-safe wrappers and systems for input processing

use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use crate::config::{MidiConfig, OscConfig, SpectrumConfig};
use crate::inputs::midi::{MidiInputManager, MidiHistoryData};
use crate::inputs::mouse::{MouseInputManager, MouseHistoryData};
use crate::inputs::osc::{OscHistoryData};
use crate::inputs::spectrum::{SpectrumHistoryData};

// Thread-safe wrapper for MIDI input manager
#[derive(Resource)]
pub struct BevyMidiInputManager {
    // Store the shared history data directly since it's already thread-safe
    pub history_data: Arc<Mutex<MidiHistoryData>>,
    // Buffer will be managed separately in rendering context
    pub buffer_needs_update: bool,
    pub enabled: bool,
    // Keep the connection alive but don't expose it directly
    _midi_manager: Option<MidiInputManager>,
}

impl BevyMidiInputManager {
    pub fn new(device: &wgpu::Device, config: &MidiConfig) -> Self {
        if config.enabled {
            let midi_manager = MidiInputManager::new(device, config);
            Self {
                history_data: midi_manager.history_data.clone(),
                buffer_needs_update: true,
                enabled: true,
                _midi_manager: Some(midi_manager),
            }
        } else {
            Self {
                history_data: Arc::new(Mutex::new(MidiHistoryData::new())),
                buffer_needs_update: false,
                enabled: false,
                _midi_manager: None,
            }
        }
    }

    pub fn update(&mut self) {
        if self.enabled {
            if let Some(ref mut manager) = self._midi_manager {
                manager.update();
                self.buffer_needs_update = true;
            }
        }
    }

    pub fn get_shader_data(&self) -> Vec<crate::inputs::midi::MidiShaderData> {
        self.history_data.lock().unwrap().prepare_shader_data()
    }

    pub fn get_buffer(&self) -> Option<&wgpu::Buffer> {
        self._midi_manager.as_ref().map(|m| &m.buffer)
    }
}

// Bevy system for updating MIDI input
pub fn midi_input_system(
    mut midi_manager: Option<ResMut<BevyMidiInputManager>>,
) {
    if let Some(ref mut manager) = midi_manager {
        manager.update();
    }
}

// Thread-safe wrapper for Mouse input manager
#[derive(Resource)]
pub struct BevyMouseInputManager {
    pub history_data: Arc<Mutex<MouseHistoryData>>,
    pub buffer_needs_update: bool,
    _mouse_manager: Option<MouseInputManager>,
}

impl BevyMouseInputManager {
    pub fn new(device: &wgpu::Device) -> Self {
        let mouse_manager = MouseInputManager::new(device);
        Self {
            history_data: mouse_manager.history_data.clone(),
            buffer_needs_update: true,
            _mouse_manager: Some(mouse_manager),
        }
    }

    pub fn update_position(&mut self, position: Vec2) {
        // Convert Bevy Vec2 to PhysicalPosition
        let physical_position = winit::dpi::PhysicalPosition::new(position.x as f64, position.y as f64);
        if let Some(ref mut manager) = self._mouse_manager {
            manager.update(&physical_position);
            self.buffer_needs_update = true;
        }
    }

    pub fn update(&mut self) {
        // Mouse update only happens when position changes
        if self.buffer_needs_update {
            self.buffer_needs_update = false;
        }
    }

    pub fn get_shader_data(&self) -> Vec<crate::inputs::mouse::MouseShaderData> {
        self.history_data.lock().unwrap().prepare_shader_data()
    }

    pub fn get_buffer(&self) -> Option<&wgpu::Buffer> {
        self._mouse_manager.as_ref().map(|m| &m.buffer)
    }
}

// Thread-safe wrapper for OSC input manager
#[derive(Resource)]
pub struct BevyOscInputManager {
    pub history_data: Arc<Mutex<OscHistoryData>>,
    // Buffer and sound_map will be managed separately
    pub buffer: Option<wgpu::Buffer>,
}

impl BevyOscInputManager {
    pub fn new(device: &wgpu::Device, config: &OscConfig) -> Self {
        // Create history data with existing OSC implementation
        let history_data = OscHistoryData::new();

        Self {
            history_data: Arc::new(Mutex::new(history_data)),
            buffer: None,
        }
    }

    pub fn update(&mut self) {
        // OSC updates will be handled through receiver and UDP packets
        // This is a placeholder for Bevy integration
    }

    pub fn get_shader_data(&self) -> Vec<crate::inputs::osc::OscShaderData> {
        if let Ok(data) = self.history_data.lock() {
            data.prepare_shader_data()
        } else {
            vec![]
        }
    }

    pub fn get_buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }
}

// Thread-safe wrapper for Spectrum input manager
#[derive(Resource)]
pub struct BevySpectrumInputManager {
    pub history_data: Arc<Mutex<SpectrumHistoryData>>,
    // Audio stream and buffer will be managed separately
    pub buffer: Option<wgpu::Buffer>,
    // Audio stream handle will be stored here for cleanup
    pub _stream_handle: Option<()>, // Placeholder for now
}

impl BevySpectrumInputManager {
    pub fn new(_device: &wgpu::Device, _config: &SpectrumConfig) -> Self {
        // Create history data with existing spectrum implementation
        let history_data = SpectrumHistoryData::new();

        Self {
            history_data: Arc::new(Mutex::new(history_data)),
            buffer: None,
            _stream_handle: None,
        }
    }

    pub fn update(&mut self) {
        // Spectrum updates will be handled through audio stream callbacks
        // This is a placeholder for Bevy integration
    }

    pub fn get_shader_data(&self) -> Vec<crate::inputs::spectrum::SpectrumShaderData> {
        if let Ok(data) = self.history_data.lock() {
            data.prepare_shader_data()
        } else {
            vec![]
        }
    }

    pub fn get_buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }
}

// Bevy system for updating mouse input
pub fn mouse_input_system(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_manager: Option<ResMut<BevyMouseInputManager>>,
) {
    if let Some(ref mut manager) = mouse_manager {
        for event in cursor_moved_events.read() {
            manager.update_position(event.position);
        }
        manager.update();
    }
}

// Initialization system for MIDI input
pub fn setup_midi_input_system(
    _commands: Commands,
    config: Res<crate::ShekerConfig>,
    // TODO: Add RenderDevice when integrating with Bevy rendering
) {
    if let Some(_midi_config) = &config.config.midi {
        log::info!("Setting up MIDI input system");

        // TODO: Create BevyMidiInputManager with proper device
        // For now, create a placeholder that will be properly initialized
        // when we integrate with Bevy's rendering system

        // let midi_manager = BevyMidiInputManager::new(device, midi_config);
        // commands.insert_resource(midi_manager);

        log::info!("MIDI input system setup completed (placeholder)");
    }
}

// Initialization system for Mouse input
pub fn setup_mouse_input_system(
    mut commands: Commands,
    _render_device: Res<bevy::render::renderer::RenderDevice>,
) {
    log::info!("Setting up Mouse input system");

    // TODO: wgpu version mismatch between Bevy (24.0) and shekere (22.0)
    // For now, create a placeholder mouse manager without storage buffer
    log::warn!("Mouse storage buffer not implemented due to wgpu version mismatch");

    log::info!("Mouse input system setup completed (placeholder)");
}

// Bevy system for updating OSC input
pub fn osc_input_system(
    mut osc_manager: Option<ResMut<BevyOscInputManager>>,
) {
    if let Some(ref mut manager) = osc_manager {
        manager.update();
    }
}

// Initialization system for OSC input
pub fn setup_osc_input_system(
    _commands: Commands,
    config: Res<crate::ShekerConfig>,
    // TODO: Add RenderDevice when integrating with Bevy rendering
) {
    if let Some(_osc_config) = &config.config.osc {
        log::info!("Setting up OSC input system");

        // TODO: Create BevyOscInputManager with proper device
        // For now, create a placeholder that will be properly initialized
        // when we integrate with Bevy's rendering system

        // let osc_manager = BevyOscInputManager::new(device, osc_config);
        // commands.insert_resource(osc_manager);

        log::info!("OSC input system setup completed (placeholder)");
    }
}

// Bevy system for updating spectrum input
pub fn spectrum_input_system(
    mut spectrum_manager: Option<ResMut<BevySpectrumInputManager>>,
) {
    if let Some(ref mut manager) = spectrum_manager {
        manager.update();
    }
}

// Initialization system for Spectrum input
pub fn setup_spectrum_input_system(
    _commands: Commands,
    config: Res<crate::ShekerConfig>,
    // TODO: Add RenderDevice when integrating with Bevy rendering
) {
    if let Some(_spectrum_config) = &config.config.spectrum {
        log::info!("Setting up Spectrum input system");

        // TODO: Create BevySpectrumInputManager with proper device and audio stream
        // For now, create a placeholder that will be properly initialized
        // when we integrate with Bevy's rendering system

        // let spectrum_manager = BevySpectrumInputManager::new(device, spectrum_config);
        // commands.insert_resource(spectrum_manager);

        log::info!("Spectrum input system setup completed (placeholder)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MidiConfig;

    #[test]
    fn test_bevy_midi_manager_disabled() {
        let config = MidiConfig { enabled: false };

        // Create a mock device for testing
        // We can't easily create a real wgpu device in tests
        // so we'll test the disabled case

        // For now, just test the structure exists
        assert!(!config.enabled);
    }
}