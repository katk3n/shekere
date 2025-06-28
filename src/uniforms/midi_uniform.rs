use std::sync::{Arc, Mutex};
use midir::{MidiInput, MidiInputConnection};
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
        let midi_in = MidiInput::new("kchfgt MIDI Input").ok()?;
        
        // Get available ports
        let in_ports = midi_in.ports();
        if in_ports.is_empty() {
            log::warn!("No MIDI input ports available");
            return None;
        }

        // Use the first available port (could be made configurable)
        let in_port = &in_ports[0];
        let port_name = midi_in.port_name(in_port).unwrap_or_else(|_| "Unknown".to_string());
        log::info!("Connecting to MIDI port: {}", port_name);

        let connection = midi_in.connect(
            in_port,
            "kchfgt-midi",
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
                        data_guard.notes[vec4_index][element_index] = velocity;
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
        // MIDI updates happen in the callback, so nothing to do here
        // The data is already updated by the MIDI input callback
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
        };
        
        // Test that all values are initialized to 0.0
        assert!(data.notes.iter().all(|vec4| vec4.iter().all(|&x| x == 0.0)));
        assert!(data.cc.iter().all(|vec4| vec4.iter().all(|&x| x == 0.0)));
    }

    #[test]
    fn test_handle_midi_note_on() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
        }));

        // Simulate Note On message: Channel 1, Note 60 (Middle C), Velocity 100
        let message = [0x90, 60, 100];
        MidiUniform::handle_midi_message(&data, &message);

        let data_guard = data.lock().unwrap();
        let vec4_index = 60 / 4; // 15
        let element_index = 60 % 4; // 0
        assert!((data_guard.notes[vec4_index][element_index] - (100.0 / 127.0)).abs() < f32::EPSILON);
        
        // Check neighboring values are still 0
        let vec4_index_59 = 59 / 4; // 14
        let element_index_59 = 59 % 4; // 3
        assert_eq!(data_guard.notes[vec4_index_59][element_index_59], 0.0);
        
        let element_index_61 = 61 % 4; // 1
        assert_eq!(data_guard.notes[vec4_index][element_index_61], 0.0);
    }

    #[test]
    fn test_handle_midi_note_off() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
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
        assert_eq!(data_guard.notes[vec4_index][element_index], 0.0);
    }

    #[test]
    fn test_handle_midi_control_change() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
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
        }));

        // Test short message
        let message = [0x90];
        MidiUniform::handle_midi_message(&data, &message);

        let data_guard = data.lock().unwrap();
        assert!(data_guard.notes.iter().all(|vec4| vec4.iter().all(|&x| x == 0.0)));
        assert!(data_guard.cc.iter().all(|vec4| vec4.iter().all(|&x| x == 0.0)));
    }

    #[test]
    fn test_handle_out_of_range_values() {
        let data = Arc::new(Mutex::new(MidiUniformData {
            notes: [[0.0; 4]; 32],
            cc: [[0.0; 4]; 32],
        }));

        // Test note number >= 128 (should be ignored)
        let message = [0x90, 128, 100];
        MidiUniform::handle_midi_message(&data, &message);

        let data_guard = data.lock().unwrap();
        assert!(data_guard.notes.iter().all(|vec4| vec4.iter().all(|&x| x == 0.0)));
    }
}