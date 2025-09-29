use midir::{MidiInput, MidiInputConnection};
#[cfg(test)]
use ringbuf::traits::Observer;
use ringbuf::{
    HeapRb,
    traits::{Consumer, RingBuffer},
};
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;

use crate::config::MidiConfig;

// Individual frame data for ring buffer storage
#[derive(Debug, Clone, Copy)]
struct MidiFrameData {
    notes: [f32; 128],
    controls: [f32; 128],
    note_on: [f32; 128],
}

impl MidiFrameData {
    fn new() -> Self {
        Self {
            notes: [0.0; 128],
            controls: [0.0; 128],
            note_on: [0.0; 128],
        }
    }

    fn clear_note_on(&mut self) {
        self.note_on = [0.0; 128];
    }
}

// Shader-compatible format for individual frame data
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MidiShaderData {
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

// History data structure using ring buffer
pub(crate) struct MidiHistoryData {
    current_frame: MidiFrameData,
    ring_buffer: HeapRb<MidiFrameData>,
}

pub struct MidiInputManager {
    pub history_data: Arc<Mutex<MidiHistoryData>>,
    pub buffer: wgpu::Buffer,
    _connection: Option<MidiInputConnection<()>>,
}

impl MidiHistoryData {
    pub fn new() -> Self {
        Self {
            current_frame: MidiFrameData::new(),
            ring_buffer: HeapRb::new(512),
        }
    }

    fn push_current_frame(&mut self) {
        // Push to ring buffer
        let _ = self.ring_buffer.push_overwrite(self.current_frame);
    }

    // Convert ring buffer data to shader-compatible linear array format
    pub fn prepare_shader_data(&self) -> Vec<MidiShaderData> {
        let mut shader_data = Vec::with_capacity(512);

        // Add current frame first (index 0 = history 0)
        shader_data.push(Self::frame_to_shader_data(&self.current_frame));

        // Add frames from ring buffer (newest to oldest)
        // Ring buffer iterator returns items in chronological order (oldest to newest),
        // so we need to collect and reverse to get newest to oldest
        let ring_data: Vec<_> = self.ring_buffer.iter().cloned().collect();
        for frame in ring_data.iter().rev() {
            shader_data.push(Self::frame_to_shader_data(frame));
            if shader_data.len() >= 512 {
                break;
            }
        }

        // Pad to exactly 512 frames if needed
        while shader_data.len() < 512 {
            shader_data.push(Self::frame_to_shader_data(&MidiFrameData::new()));
        }

        shader_data
    }

    fn frame_to_shader_data(frame: &MidiFrameData) -> MidiShaderData {
        let mut notes = [[0.0f32; 4]; 32];
        let mut cc = [[0.0f32; 4]; 32];
        let mut note_on = [[0.0f32; 4]; 32];

        // Pack notes into vec4 format
        for i in 0..128 {
            let vec4_index = i / 4;
            let element_index = i % 4;
            notes[vec4_index][element_index] = frame.notes[i];
            cc[vec4_index][element_index] = frame.controls[i];
            note_on[vec4_index][element_index] = frame.note_on[i];
        }

        MidiShaderData { notes, cc, note_on }
    }
}

impl MidiInputManager {
    pub const BINDING_INDEX: u32 = 2;

    pub fn new(device: &wgpu::Device, config: &MidiConfig) -> Self {
        let history_data = Arc::new(Mutex::new(MidiHistoryData::new()));

        // Calculate buffer size for 512 frames (768KB total)
        let single_frame_size = std::mem::size_of::<MidiShaderData>();
        let _total_size = single_frame_size * 512; // 768KB total

        // Create initial shader data
        let initial_shader_data = history_data.lock().unwrap().prepare_shader_data();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MIDI History Buffer"),
            contents: bytemuck::cast_slice(&initial_shader_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let connection = if config.enabled {
            Self::setup_midi_input(history_data.clone())
        } else {
            None
        };

        Self {
            history_data,
            buffer,
            _connection: connection,
        }
    }

    fn setup_midi_input(
        history_data: Arc<Mutex<MidiHistoryData>>,
    ) -> Option<MidiInputConnection<()>> {
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
                Self::handle_midi_message(&history_data, message);
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

    fn handle_midi_message(history_data: &Arc<Mutex<MidiHistoryData>>, message: &[u8]) {
        if message.len() < 2 {
            return;
        }

        let mut history_guard = history_data.lock().unwrap();
        let current_frame = &mut history_guard.current_frame;

        match message[0] & 0xF0 {
            // Note On (0x90)
            0x90 => {
                if message.len() >= 3 {
                    let note = message[1] as usize;
                    let velocity = message[2] as f32 / 127.0;
                    if note < 128 {
                        // Set sustained note
                        current_frame.notes[note] = velocity;
                        // Set attack detection
                        current_frame.note_on[note] = velocity;
                    }
                }
            }
            // Note Off (0x80)
            0x80 => {
                if message.len() >= 3 {
                    let note = message[1] as usize;
                    if note < 128 {
                        current_frame.notes[note] = 0.0;
                    }
                }
            }
            // Control Change (0xB0)
            0xB0 => {
                if message.len() >= 3 {
                    let controller = message[1] as usize;
                    let value = message[2] as f32 / 127.0;
                    if controller < 128 {
                        current_frame.controls[controller] = value;
                    }
                }
            }
            _ => {
                // Ignore other message types for now
            }
        }
    }

    pub fn update(&mut self) {
        let mut history_guard = self.history_data.lock().unwrap();

        // Push current frame to ring buffer for history
        history_guard.push_current_frame();

        // Clear note_on array for next frame (attack detection)
        history_guard.current_frame.clear_note_on();
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        let history_guard = self.history_data.lock().unwrap();
        let shader_data = history_guard.prepare_shader_data();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&shader_data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_frame_data_creation() {
        let frame = MidiFrameData::new();

        // Test that all values are initialized to 0.0
        assert!(frame.notes.iter().all(|&x| x == 0.0));
        assert!(frame.controls.iter().all(|&x| x == 0.0));
        assert!(frame.note_on.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_midi_history_data_creation() {
        let history = MidiHistoryData::new();

        // Test that current frame is initialized to zeros
        assert!(history.current_frame.notes.iter().all(|&x| x == 0.0));
        assert!(history.current_frame.controls.iter().all(|&x| x == 0.0));
        assert!(history.current_frame.note_on.iter().all(|&x| x == 0.0));

        // Test that ring buffer is empty and properly sized
        assert_eq!(history.ring_buffer.occupied_len(), 0);
        assert_eq!(history.ring_buffer.capacity().get(), 512);
    }

    #[test]
    fn test_ring_buffer_push_and_observe() {
        let mut history = MidiHistoryData::new();

        // Set some values in current frame
        history.current_frame.notes[60] = 0.8; // Middle C
        history.current_frame.controls[7] = 0.5; // Volume
        history.current_frame.note_on[64] = 0.9; // E

        // Push current frame to ring buffer
        history.push_current_frame();

        // Ring buffer should now have 1 frame
        assert_eq!(history.ring_buffer.occupied_len(), 1);

        // Check the frame data from ring buffer
        let ring_data: Vec<_> = history.ring_buffer.iter().cloned().collect();
        assert_eq!(ring_data.len(), 1);
        let frame = &ring_data[0];
        assert_eq!(frame.notes[60], 0.8);
        assert_eq!(frame.controls[7], 0.5);
        assert_eq!(frame.note_on[64], 0.9);
    }

    #[test]
    fn test_ring_buffer_overwrite_behavior() {
        let mut history = MidiHistoryData::new();

        // Fill ring buffer beyond capacity
        for i in 0..600 {
            history.current_frame.notes[0] = (i as f32) / 600.0;
            history.push_current_frame();
        }

        // Ring buffer should be full (512 frames)
        assert_eq!(history.ring_buffer.occupied_len(), 512);

        // Check the frame data from ring buffer
        let ring_data: Vec<_> = history.ring_buffer.iter().cloned().collect();
        assert_eq!(ring_data.len(), 512);

        // The first frame in ring buffer should be from iteration 88 (600 - 512)
        // and the last frame should be from iteration 599
        let expected_first_value = 88.0 / 600.0;
        let expected_last_value = 599.0 / 600.0;
        assert!((ring_data[0].notes[0] - expected_first_value).abs() < f32::EPSILON);
        assert!((ring_data[511].notes[0] - expected_last_value).abs() < f32::EPSILON);
    }

    #[test]
    fn test_frame_to_shader_data_conversion() {
        let mut frame = MidiFrameData::new();
        frame.notes[60] = 0.8; // Middle C
        frame.controls[7] = 0.5; // Volume CC
        frame.note_on[64] = 0.9; // E attack

        let shader_data = MidiHistoryData::frame_to_shader_data(&frame);

        // Check that values are correctly packed into vec4 format
        let note_vec4_index = 60 / 4; // 15
        let note_element_index = 60 % 4; // 0
        assert_eq!(shader_data.notes[note_vec4_index][note_element_index], 0.8);

        let cc_vec4_index = 7 / 4; // 1
        let cc_element_index = 7 % 4; // 3
        assert_eq!(shader_data.cc[cc_vec4_index][cc_element_index], 0.5);

        let note_on_vec4_index = 64 / 4; // 16
        let note_on_element_index = 64 % 4; // 0
        assert_eq!(
            shader_data.note_on[note_on_vec4_index][note_on_element_index],
            0.9
        );
    }

    #[test]
    fn test_prepare_shader_data() {
        let mut history = MidiHistoryData::new();

        // Add some frames to history using push_current_frame
        for i in 0..3 {
            let mut frame = MidiFrameData::new();
            frame.notes[60] = (i as f32) * 0.1;
            history.current_frame = frame;
            history.push_current_frame();
        }

        // Set final current frame values
        history.current_frame.notes[60] = 1.0;
        history.current_frame.controls[7] = 0.8;

        let shader_data = history.prepare_shader_data();

        // Should have exactly 512 frames
        assert_eq!(shader_data.len(), 512);

        // First frame (index 0) should be current frame
        let first_frame = &shader_data[0];
        let note_vec4_index = 60 / 4;
        let note_element_index = 60 % 4;
        assert_eq!(first_frame.notes[note_vec4_index][note_element_index], 1.0);

        let cc_vec4_index = 7 / 4;
        let cc_element_index = 7 % 4;
        assert_eq!(first_frame.cc[cc_vec4_index][cc_element_index], 0.8);

        // Following frames should be from ring buffer (newest first)
        let second_frame = &shader_data[1];
        assert_eq!(second_frame.notes[note_vec4_index][note_element_index], 0.2); // Last pushed frame (index 2)

        let third_frame = &shader_data[2];
        assert_eq!(third_frame.notes[note_vec4_index][note_element_index], 0.1); // Second frame (index 1)

        let fourth_frame = &shader_data[3];
        assert_eq!(fourth_frame.notes[note_vec4_index][note_element_index], 0.0); // First frame (index 0)

        // Remaining frames should be zeros
        for i in 4..512 {
            let frame = &shader_data[i];
            assert!(
                frame
                    .notes
                    .iter()
                    .all(|vec4| vec4.iter().all(|&x| x == 0.0))
            );
        }
    }

    #[test]
    fn test_ring_buffer_only_history() {
        let mut history = MidiHistoryData::new();

        // Create frames and push them through ring buffer
        let mut frames = Vec::new();
        for i in 0..5 {
            let mut frame = MidiFrameData::new();
            frame.notes[60] = (i as f32) * 0.1;
            frame.controls[7] = (i as f32) * 0.2;
            frames.push(frame);

            // Set as current frame and push to ring buffer
            history.current_frame = frame;
            history.push_current_frame();
        }

        // Verify ring buffer contains the correct data
        assert_eq!(history.ring_buffer.occupied_len(), 5);

        // Ring buffer should contain frames in chronological order
        let ring_data: Vec<_> = history.ring_buffer.iter().cloned().collect();
        for (i, frame) in ring_data.iter().enumerate() {
            assert_eq!(frame.notes[60], (i as f32) * 0.1);
            assert_eq!(frame.controls[7], (i as f32) * 0.2);
        }

        // Current frame should be the last one we set
        assert_eq!(history.current_frame.notes[60], 0.4);
        assert_eq!(history.current_frame.controls[7], 0.8);
    }

    #[test]
    fn test_ring_buffer_overwrite() {
        let mut history = MidiHistoryData::new();

        // Fill the ring buffer beyond capacity (512 + 10 = 522 frames)
        for i in 0..522 {
            let mut frame = MidiFrameData::new();
            frame.notes[60] = (i % 256) as f32 / 255.0; // Use modulo to avoid large values
            history.current_frame = frame;
            history.push_current_frame();
        }

        // Ring buffer should be at full capacity
        assert_eq!(history.ring_buffer.occupied_len(), 512);

        // The oldest frames should have been overwritten
        // The first frame in the ring buffer should now be frame 10 (index 10)
        let ring_data: Vec<_> = history.ring_buffer.iter().cloned().collect();
        assert_eq!(ring_data[0].notes[60], 10.0 / 255.0);

        // The last frame should be frame 521 (index 521)
        assert_eq!(ring_data[511].notes[60], (521 % 256) as f32 / 255.0);
    }

    #[test]
    fn test_handle_midi_note_on() {
        let history_data = Arc::new(Mutex::new(MidiHistoryData::new()));

        // Simulate Note On message: Channel 1, Note 60 (Middle C), Velocity 100
        let message = [0x90, 60, 100];
        MidiInputManager::handle_midi_message(&history_data, &message);

        let history_guard = history_data.lock().unwrap();
        let expected_velocity = 100.0 / 127.0;

        // Test that both notes and note_on arrays are set
        assert!((history_guard.current_frame.notes[60] - expected_velocity).abs() < f32::EPSILON);
        assert!((history_guard.current_frame.note_on[60] - expected_velocity).abs() < f32::EPSILON);

        // Check neighboring values are still 0
        assert_eq!(history_guard.current_frame.notes[59], 0.0);
        assert_eq!(history_guard.current_frame.note_on[59], 0.0);
        assert_eq!(history_guard.current_frame.notes[61], 0.0);
        assert_eq!(history_guard.current_frame.note_on[61], 0.0);
    }

    #[test]
    fn test_handle_midi_note_off() {
        let history_data = Arc::new(Mutex::new(MidiHistoryData::new()));

        // First set a note on
        let note_on = [0x90, 60, 100];
        MidiInputManager::handle_midi_message(&history_data, &note_on);

        // Then turn it off
        let note_off = [0x80, 60, 0];
        MidiInputManager::handle_midi_message(&history_data, &note_off);

        let history_guard = history_data.lock().unwrap();
        // Note off should clear notes array but preserve note_on
        assert_eq!(history_guard.current_frame.notes[60], 0.0);
        assert!((history_guard.current_frame.note_on[60] - (100.0 / 127.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_handle_midi_control_change() {
        let history_data = Arc::new(Mutex::new(MidiHistoryData::new()));

        // Simulate Control Change message: Channel 1, Controller 7 (Volume), Value 64
        let message = [0xB0, 7, 64];
        MidiInputManager::handle_midi_message(&history_data, &message);

        let history_guard = history_data.lock().unwrap();
        assert!((history_guard.current_frame.controls[7] - (64.0 / 127.0)).abs() < f32::EPSILON);

        // Check neighboring values are still 0
        assert_eq!(history_guard.current_frame.controls[6], 0.0);
        assert_eq!(history_guard.current_frame.controls[8], 0.0);
    }

    #[test]
    fn test_handle_invalid_midi_message() {
        let history_data = Arc::new(Mutex::new(MidiHistoryData::new()));

        // Test short message
        let message = [0x90];
        MidiInputManager::handle_midi_message(&history_data, &message);

        let history_guard = history_data.lock().unwrap();
        assert!(history_guard.current_frame.notes.iter().all(|&x| x == 0.0));
        assert!(
            history_guard
                .current_frame
                .controls
                .iter()
                .all(|&x| x == 0.0)
        );
    }

    #[test]
    fn test_handle_out_of_range_values() {
        let history_data = Arc::new(Mutex::new(MidiHistoryData::new()));

        // Test note number >= 128 (should be ignored)
        let message = [0x90, 128, 100];
        MidiInputManager::handle_midi_message(&history_data, &message);

        let history_guard = history_data.lock().unwrap();
        assert!(history_guard.current_frame.notes.iter().all(|&x| x == 0.0));
        assert!(
            history_guard
                .current_frame
                .note_on
                .iter()
                .all(|&x| x == 0.0)
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
        let mut midi_input_manager = MidiInputManager::new(&device, &config);

        // Set note_on value directly
        {
            let mut history_guard = midi_input_manager.history_data.lock().unwrap();
            history_guard.current_frame.note_on[60] = 0.5; // Set middle C attack
        }

        // Call update (should clear note_on and push to history)
        midi_input_manager.update();

        // Verify note_on was cleared and frame was pushed to history
        let history_guard = midi_input_manager.history_data.lock().unwrap();
        assert_eq!(history_guard.current_frame.note_on[60], 0.0);
        assert!(
            history_guard
                .current_frame
                .note_on
                .iter()
                .all(|&x| x == 0.0)
        );

        // Verify frame was pushed to ring buffer
        assert_eq!(history_guard.ring_buffer.occupied_len(), 1);
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
        let mut midi_input_manager = MidiInputManager::new(&device, &config);

        // Simulate Note On message processing
        let note_on_message = [0x90, 60, 100]; // Channel 1, Middle C, Velocity 100
        MidiInputManager::handle_midi_message(&midi_input_manager.history_data, &note_on_message);

        // First frame: attack should be detected
        {
            let history_guard = midi_input_manager.history_data.lock().unwrap();
            let expected_velocity = 100.0 / 127.0;

            // Both notes and note_on should be set
            assert!(
                (history_guard.current_frame.notes[60] - expected_velocity).abs() < f32::EPSILON,
                "Notes array should contain velocity"
            );
            assert!(
                (history_guard.current_frame.note_on[60] - expected_velocity).abs() < f32::EPSILON,
                "Note_on array should contain attack velocity"
            );
        }

        // Simulate frame update (clears note_on and pushes to history)
        midi_input_manager.update();

        // Second frame: attack should be cleared, sustained note remains
        {
            let history_guard = midi_input_manager.history_data.lock().unwrap();
            let expected_velocity = 100.0 / 127.0;

            // Notes should still be set (sustained)
            assert!(
                (history_guard.current_frame.notes[60] - expected_velocity).abs() < f32::EPSILON,
                "Notes array should still contain velocity after frame update"
            );
            // Note_on should be cleared
            assert_eq!(
                history_guard.current_frame.note_on[60], 0.0,
                "Note_on array should be cleared after frame update"
            );

            // Ring buffer should contain the previous frame
            assert_eq!(history_guard.ring_buffer.occupied_len(), 1);
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
        let mut midi_input_manager = MidiInputManager::new(&device, &config);

        // Send multiple Note On messages
        let notes = [
            (60, 100), // Middle C, velocity 100
            (64, 80),  // E, velocity 80
            (67, 120), // G, velocity 120
        ];

        for (note, velocity) in notes.iter() {
            let message = [0x90, *note, *velocity];
            MidiInputManager::handle_midi_message(&midi_input_manager.history_data, &message);
        }

        // Verify all notes are detected
        {
            let history_guard = midi_input_manager.history_data.lock().unwrap();
            for (note, velocity) in notes.iter() {
                let expected_velocity = *velocity as f32 / 127.0;

                assert!(
                    (history_guard.current_frame.notes[*note as usize] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Note {} should have velocity {}",
                    note,
                    expected_velocity
                );
                assert!(
                    (history_guard.current_frame.note_on[*note as usize] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Note {} should have attack velocity {}",
                    note,
                    expected_velocity
                );
            }
        }

        // Clear attacks and push to history
        midi_input_manager.update();

        // Verify attacks cleared but sustained notes remain
        {
            let history_guard = midi_input_manager.history_data.lock().unwrap();
            for (note, velocity) in notes.iter() {
                let expected_velocity = *velocity as f32 / 127.0;

                assert!(
                    (history_guard.current_frame.notes[*note as usize] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Sustained note {} should remain after frame update",
                    note
                );
                assert_eq!(
                    history_guard.current_frame.note_on[*note as usize], 0.0,
                    "Attack for note {} should be cleared after frame update",
                    note
                );
            }

            // Ring buffer should contain the previous frame
            assert_eq!(history_guard.ring_buffer.occupied_len(), 1);
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
        let midi_input_manager = MidiInputManager::new(&device, &config);

        // Send a C major chord simultaneously (C-E-G)
        let chord_notes = [60, 64, 67]; // C, E, G
        let velocity = 100;

        // Send all notes in same frame
        for note in chord_notes.iter() {
            let message = [0x90, *note, velocity];
            MidiInputManager::handle_midi_message(&midi_input_manager.history_data, &message);
        }

        // Verify all chord notes detected simultaneously
        {
            let history_guard = midi_input_manager.history_data.lock().unwrap();
            let expected_velocity = velocity as f32 / 127.0;

            for note in chord_notes.iter() {
                assert!(
                    (history_guard.current_frame.note_on[*note as usize] - expected_velocity).abs()
                        < f32::EPSILON,
                    "Chord note {} should be detected with attack",
                    note
                );
            }
        }
    }
}
