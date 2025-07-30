use midir::{MidiInput, MidiInputConnection};
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;

use crate::config::MidiConfig;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MidiUniformData {
    // note velocities (0-127 normalized to 0.0-1.0)
    // Using vec4<f32> for alignment (16-byte alignment)
    notes: [[f32; 4]; 32], // 128 notes packed into 32 vec4s
    // control change values (0-127 normalized to 0.0-1.0)
    // Using vec4<f32> for alignment (16-byte alignment)
    cc: [[f32; 4]; 32], // 128 cc values packed into 32 vec4s
    // note on attack detection (0-127 normalized to 0.0-1.0)
    // Using vec4<f32> for alignment (16-byte alignment)
    note_on: [[f32; 4]; 32], // 128 note on values packed into 32 vec4s
}

pub struct MidiUniform {
    pub data: Arc<Mutex<MidiUniformData>>,
    pub buffer: wgpu::Buffer,
    _connection: Option<MidiInputConnection<()>>,
}

impl MidiUniform {
    pub const BINDING_INDEX: u32 = 2;

    pub fn new(device: &wgpu::Device, config: &MidiConfig) -> Self {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
            note_on: [[0.0; 4]; 32],
        }));

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MIDI Buffer"),
            contents: bytemuck::cast_slice(&[*data.lock().unwrap()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let connection = if config.enabled {
            Self::setup_midi_input(data.clone())
        } else {
            None
        };

        Self {
            data,
            buffer,
            _connection: connection,
        }
    }

    fn setup_midi_input(data: Arc<Mutex<MidiUniformData>>) -> Option<MidiInputConnection<()>> {
        let midi_in = MidiInput::new("shekere MIDI Input").ok()?;

        // Get available ports
        let in_ports = midi_in.ports();
        if in_ports.is_empty() {
            log::warn!("No MIDI input ports available");
            return None;
        }

        // Use the first available port (could be made configurable)
        let in_port = &in_ports[0];
        let port_name = midi_in
            .port_name(in_port)
            .unwrap_or_else(|_| "Unknown".to_string());
        log::info!("Connecting to MIDI port: {}", port_name);

        let connection = midi_in.connect(
            in_port,
            "shekere-midi",
            move |_timestamp, message, _| {
                Self::handle_midi_message(&data, message);
            },
            (),
        );

        match connection {
            Ok(conn) => {
                log::info!("MIDI input connected successfully");
                Some(conn)
            }
            Err(e) => {
                log::error!("Failed to connect MIDI input: {}", e);
                None
            }
        }
    }

    fn handle_midi_message(data: &Arc<Mutex<MidiUniformData>>, message: &[u8]) {
        if message.len() < 2 {
            return;
        }

        let mut data_guard = data.lock().unwrap();

        match message[0] & 0xF0 {
            // Note On (0x90)
            0x90 => {
                if message.len() >= 3 {
                    let note = message[1] as usize;
                    let velocity = message[2] as f32 / 127.0;
                    if note < 128 {
                        let vec4_index = note / 4;
                        let element_index = note % 4;
                        // Set sustained note
                        data_guard.notes[vec4_index][element_index] = velocity;
                        // Set attack detection
                        data_guard.note_on[vec4_index][element_index] = velocity;
                    }
                }
            }
            // Note Off (0x80)
            0x80 => {
                if message.len() >= 3 {
                    let note = message[1] as usize;
                    if note < 128 {
                        let vec4_index = note / 4;
                        let element_index = note % 4;
                        data_guard.notes[vec4_index][element_index] = 0.0;
                    }
                }
            }
            // Control Change (0xB0)
            0xB0 => {
                if message.len() >= 3 {
                    let controller = message[1] as usize;
                    let value = message[2] as f32 / 127.0;
                    if controller < 128 {
                        let vec4_index = controller / 4;
                        let element_index = controller % 4;
                        data_guard.cc[vec4_index][element_index] = value;
                    }
                }
            }
            _ => {
                // Ignore other message types for now
            }
        }
    }

    pub fn update(&mut self) {
        // Clear note_on array at frame start
        let mut data = self.data.lock().unwrap();
        data.note_on = [[0.0; 4]; 32];
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        let data = *self.data.lock().unwrap();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_uniform_data_creation() {
        let data = MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
            note_on: [[0.0; 4]; 32],
        };

        // Test that all values are initialized to 0.0
        assert!(data.notes.iter().all(|vec4| vec4.iter().all(|&x| x == 0.0)));
        assert!(data.cc.iter().all(|vec4| vec4.iter().all(|&x| x == 0.0)));
        assert!(
            data.note_on
                .iter()
                .all(|vec4| vec4.iter().all(|&x| x == 0.0))
        );
    }

    #[test]
    fn test_handle_midi_note_on() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
            note_on: [[0.0; 4]; 32],
        }));

        // Simulate Note On message: Channel 1, Note 60 (Middle C), Velocity 100
        let message = [0x90, 60, 100];
        MidiUniform::handle_midi_message(&data, &message);

        let data_guard = data.lock().unwrap();
        let vec4_index = 60 / 4; // 15
        let element_index = 60 % 4; // 0
        let expected_velocity = 100.0 / 127.0;

        // Test that both notes and note_on arrays are set
        assert!(
            (data_guard.notes[vec4_index][element_index] - expected_velocity).abs() < f32::EPSILON
        );
        assert!(
            (data_guard.note_on[vec4_index][element_index] - expected_velocity).abs()
                < f32::EPSILON
        );

        // Check neighboring values are still 0
        let vec4_index_59 = 59 / 4; // 14
        let element_index_59 = 59 % 4; // 3
        assert_eq!(data_guard.notes[vec4_index_59][element_index_59], 0.0);
        assert_eq!(data_guard.note_on[vec4_index_59][element_index_59], 0.0);

        let element_index_61 = 61 % 4; // 1
        assert_eq!(data_guard.notes[vec4_index][element_index_61], 0.0);
        assert_eq!(data_guard.note_on[vec4_index][element_index_61], 0.0);
    }

    #[test]
    fn test_handle_midi_note_off() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
            note_on: [[0.0; 4]; 32],
        }));

        // First set a note on
        let note_on = [0x90, 60, 100];
        MidiUniform::handle_midi_message(&data, &note_on);

        // Then turn it off
        let note_off = [0x80, 60, 0];
        MidiUniform::handle_midi_message(&data, &note_off);

        let data_guard = data.lock().unwrap();
        let vec4_index = 60 / 4;
        let element_index = 60 % 4;
        // Note off should clear notes array but preserve note_on
        assert_eq!(data_guard.notes[vec4_index][element_index], 0.0);
        assert!(
            (data_guard.note_on[vec4_index][element_index] - (100.0 / 127.0)).abs() < f32::EPSILON
        );
    }

    #[test]
    fn test_handle_midi_control_change() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
            note_on: [[0.0; 4]; 32],
        }));

        // Simulate Control Change message: Channel 1, Controller 7 (Volume), Value 64
        let message = [0xB0, 7, 64];
        MidiUniform::handle_midi_message(&data, &message);

        let data_guard = data.lock().unwrap();
        let vec4_index = 7 / 4; // 1
        let element_index = 7 % 4; // 3
        assert!((data_guard.cc[vec4_index][element_index] - (64.0 / 127.0)).abs() < f32::EPSILON);

        let vec4_index_6 = 6 / 4; // 1
        let element_index_6 = 6 % 4; // 2
        assert_eq!(data_guard.cc[vec4_index_6][element_index_6], 0.0);

        let vec4_index_8 = 8 / 4; // 2
        let element_index_8 = 8 % 4; // 0
        assert_eq!(data_guard.cc[vec4_index_8][element_index_8], 0.0);
    }

    #[test]
    fn test_handle_invalid_midi_message() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
            note_on: [[0.0; 4]; 32],
        }));

        // Test short message
        let message = [0x90];
        MidiUniform::handle_midi_message(&data, &message);

        let data_guard = data.lock().unwrap();
        assert!(
            data_guard
                .notes
                .iter()
                .all(|vec4| vec4.iter().all(|&x| x == 0.0))
        );
        assert!(
            data_guard
                .cc
                .iter()
                .all(|vec4| vec4.iter().all(|&x| x == 0.0))
        );
    }

    #[test]
    fn test_handle_out_of_range_values() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
            note_on: [[0.0; 4]; 32],
        }));

        // Test note number >= 128 (should be ignored)
        let message = [0x90, 128, 100];
        MidiUniform::handle_midi_message(&data, &message);

        let data_guard = data.lock().unwrap();
        assert!(
            data_guard
                .notes
                .iter()
                .all(|vec4| vec4.iter().all(|&x| x == 0.0))
        );
        assert!(
            data_guard
                .note_on
                .iter()
                .all(|vec4| vec4.iter().all(|&x| x == 0.0))
        );
    }

    #[test]
    fn test_note_on_frame_clearing() {
        use crate::config::MidiConfig;

        // Create a device for testing (using wgpu test utilities)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, _queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .unwrap();

        let config = MidiConfig { enabled: false };
        let mut midi_uniform = MidiUniform::new(&device, &config);

        // Set note_on value directly
        {
            let mut data = midi_uniform.data.lock().unwrap();
            data.note_on[15][0] = 0.5; // Set middle C attack
        }

        // Call update (should clear note_on)
        midi_uniform.update();

        // Verify note_on was cleared
        let data = midi_uniform.data.lock().unwrap();
        assert_eq!(data.note_on[15][0], 0.0);
        assert!(
            data.note_on
                .iter()
                .all(|vec4| vec4.iter().all(|&x| x == 0.0))
        );
    }

    #[test]
    fn test_end_to_end_note_on_detection() {
        use crate::config::MidiConfig;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, _queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .unwrap();

        let config = MidiConfig { enabled: false };
        let mut midi_uniform = MidiUniform::new(&device, &config);

        // Simulate Note On message processing
        let note_on_message = [0x90, 60, 100]; // Channel 1, Middle C, Velocity 100
        MidiUniform::handle_midi_message(&midi_uniform.data, &note_on_message);

        // First frame: attack should be detected
        {
            let data = midi_uniform.data.lock().unwrap();
            let vec4_index = 60 / 4; // 15
            let element_index = 60 % 4; // 0
            let expected_velocity = 100.0 / 127.0;

            // Both notes and note_on should be set
            assert!(
                (data.notes[vec4_index][element_index] - expected_velocity).abs() < f32::EPSILON,
                "Notes array should contain velocity"
            );
            assert!(
                (data.note_on[vec4_index][element_index] - expected_velocity).abs() < f32::EPSILON,
                "Note_on array should contain attack velocity"
            );
        }

        // Simulate frame update (clears note_on)
        midi_uniform.update();

        // Second frame: attack should be cleared, sustained note remains
        {
            let data = midi_uniform.data.lock().unwrap();
            let vec4_index = 60 / 4;
            let element_index = 60 % 4;
            let expected_velocity = 100.0 / 127.0;

            // Notes should still be set (sustained)
            assert!(
                (data.notes[vec4_index][element_index] - expected_velocity).abs() < f32::EPSILON,
                "Notes array should still contain velocity after frame update"
            );
            // Note_on should be cleared
            assert_eq!(
                data.note_on[vec4_index][element_index], 0.0,
                "Note_on array should be cleared after frame update"
            );
        }
    }

    #[test]
    fn test_multiple_note_on_events() {
        use crate::config::MidiConfig;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, _queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .unwrap();

        let config = MidiConfig { enabled: false };
        let mut midi_uniform = MidiUniform::new(&device, &config);

        // Send multiple Note On messages
        let notes = [
            (60, 100), // Middle C, velocity 100
            (64, 80),  // E, velocity 80
            (67, 120), // G, velocity 120
        ];

        for (note, velocity) in notes.iter() {
            let message = [0x90, *note, *velocity];
            MidiUniform::handle_midi_message(&midi_uniform.data, &message);
        }

        // Verify all notes are detected
        {
            let data = midi_uniform.data.lock().unwrap();
            for (note, velocity) in notes.iter() {
                let vec4_index = *note as usize / 4;
                let element_index = *note as usize % 4;
                let expected_velocity = *velocity as f32 / 127.0;

                assert!(
                    (data.notes[vec4_index][element_index] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Note {} should have velocity {}",
                    note,
                    expected_velocity
                );
                assert!(
                    (data.note_on[vec4_index][element_index] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Note {} should have attack velocity {}",
                    note,
                    expected_velocity
                );
            }
        }

        // Clear attacks
        midi_uniform.update();

        // Verify attacks cleared but sustained notes remain
        {
            let data = midi_uniform.data.lock().unwrap();
            for (note, velocity) in notes.iter() {
                let vec4_index = *note as usize / 4;
                let element_index = *note as usize % 4;
                let expected_velocity = *velocity as f32 / 127.0;

                assert!(
                    (data.notes[vec4_index][element_index] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Sustained note {} should remain after frame update",
                    note
                );
                assert_eq!(
                    data.note_on[vec4_index][element_index], 0.0,
                    "Attack for note {} should be cleared after frame update",
                    note
                );
            }
        }
    }

    #[test]
    fn test_chord_detection() {
        use crate::config::MidiConfig;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, _queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .unwrap();

        let config = MidiConfig { enabled: false };
        let midi_uniform = MidiUniform::new(&device, &config);

        // Send a C major chord simultaneously (C-E-G)
        let chord_notes = [60, 64, 67]; // C, E, G
        let velocity = 100;

        // Send all notes in same frame
        for note in chord_notes.iter() {
            let message = [0x90, *note, velocity];
            MidiUniform::handle_midi_message(&midi_uniform.data, &message);
        }

        // Verify all chord notes detected simultaneously
        {
            let data = midi_uniform.data.lock().unwrap();
            let expected_velocity = velocity as f32 / 127.0;

            for note in chord_notes.iter() {
                let vec4_index = *note as usize / 4;
                let element_index = *note as usize % 4;

                assert!(
                    (data.note_on[vec4_index][element_index] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Chord note {} should be detected with attack",
                    note
                );
            }
        }
    }
}
