use async_std::channel::{Receiver, unbounded};
use async_std::net::{SocketAddrV4, UdpSocket};
use async_std::task;
use bevy::render::render_resource::ShaderType;
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, RingBuffer},
};
use rosc::{OscMessage, OscPacket, OscType};
use std::collections::HashMap;
use std::str::FromStr;

const HISTORY_SIZE: usize = 512;

// OSC server setup function
async fn osc_start(port: u32) -> Receiver<OscPacket> {
    let addr = match SocketAddrV4::from_str(&format!("0.0.0.0:{}", port)) {
        Ok(addr) => addr,
        Err(_) => panic!("Error"),
    };
    let sock = UdpSocket::bind(addr).await.unwrap();
    log::info!("[OSC] Listening to {}", addr);
    let mut buf = [0u8; rosc::decoder::MTU];
    let (sender, receiver) = unbounded();
    task::spawn(async move {
        loop {
            let (size, _addr) = sock.recv_from(&mut buf).await.unwrap();
            let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
            let _ = sender.send(packet).await;
        }
    });

    receiver
}

// Individual frame data for ring buffer storage
#[derive(Debug, Clone, Copy)]
pub(crate) struct OscFrameData {
    pub sounds: [i32; 16],
    pub ttls: [f32; 16],
    pub notes: [f32; 16],
    pub gains: [f32; 16],
}

impl Default for OscFrameData {
    fn default() -> Self {
        Self {
            sounds: [0; 16],
            ttls: [0.0; 16],
            notes: [0.0; 16],
            gains: [0.0; 16],
        }
    }
}

// GPU-aligned data structure for storage buffer
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, ShaderType)]
pub struct OscShaderData {
    // Packed into vec4s for WebGPU alignment (16 values / 4 = 4 vec4s each)
    pub sounds: [[i32; 4]; 4],
    pub ttls: [[f32; 4]; 4],
    pub notes: [[f32; 4]; 4],
    pub gains: [[f32; 4]; 4],
}

impl From<OscFrameData> for OscShaderData {
    fn from(frame: OscFrameData) -> Self {
        let mut sounds = [[0; 4]; 4];
        let mut ttls = [[0.0; 4]; 4];
        let mut notes = [[0.0; 4]; 4];
        let mut gains = [[0.0; 4]; 4];

        // Pack arrays into vec4s for GPU alignment
        for i in 0..4 {
            for j in 0..4 {
                let idx = i * 4 + j;
                sounds[i][j] = frame.sounds[idx];
                ttls[i][j] = frame.ttls[idx];
                notes[i][j] = frame.notes[idx];
                gains[i][j] = frame.gains[idx];
            }
        }

        Self {
            sounds,
            ttls,
            notes,
            gains,
        }
    }
}

// History data structure using ring buffer only (optimized)
pub(crate) struct OscHistoryData {
    pub current_frame: OscFrameData,
    pub ring_buffer: HeapRb<OscFrameData>,
}

impl OscHistoryData {
    pub fn new() -> Self {
        Self {
            current_frame: OscFrameData::default(),
            ring_buffer: HeapRb::new(HISTORY_SIZE),
        }
    }

    pub fn update_sound(&mut self, index: usize, value: i32) {
        if index < 16 {
            self.current_frame.sounds[index] = value;
        }
    }

    pub fn update_ttl(&mut self, index: usize, value: f32) {
        if index < 16 {
            self.current_frame.ttls[index] = value;
        }
    }

    pub fn update_note(&mut self, index: usize, value: f32) {
        if index < 16 {
            self.current_frame.notes[index] = value;
        }
    }

    pub fn update_gain(&mut self, index: usize, value: f32) {
        if index < 16 {
            self.current_frame.gains[index] = value;
        }
    }

    pub fn push_current_frame(&mut self) {
        // O(1) operation - ring buffer handles overflow automatically
        self.ring_buffer.push_overwrite(self.current_frame);
    }

    pub fn prepare_shader_data(&self) -> Vec<OscShaderData> {
        let mut shader_data = Vec::with_capacity(HISTORY_SIZE);

        // First add current frame (index 0)
        shader_data.push(self.current_frame.into());

        // Then add historical frames from ring buffer (newest to oldest)
        let _occupied_len = self.ring_buffer.occupied_len();
        for frame_data in self.ring_buffer.iter().take(HISTORY_SIZE - 1) {
            shader_data.push((*frame_data).into());
        }

        // Fill remaining slots with default data if needed
        while shader_data.len() < HISTORY_SIZE {
            shader_data.push(OscFrameData::default().into());
        }

        shader_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_frame_data_creation() {
        let frame = OscFrameData::default();
        assert_eq!(frame.sounds[0], 0);
        assert_eq!(frame.ttls[0], 0.0);
        assert_eq!(frame.notes[0], 0.0);
        assert_eq!(frame.gains[0], 0.0);
    }

    #[test]
    fn test_osc_shader_data_gpu_alignment() {
        // Verify OscShaderData has correct size for GPU alignment
        assert_eq!(std::mem::size_of::<OscShaderData>(), 256); // 4 * 4 * 4 * 4 bytes
        assert_eq!(std::mem::align_of::<OscShaderData>(), 4);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn test_osc_frame_to_shader_data_conversion() {
        let mut frame = OscFrameData::default();
        frame.sounds[5] = 42;
        frame.ttls[10] = 3.14;
        frame.notes[15] = 440.0;
        frame.gains[7] = 0.8;

        let shader_data: OscShaderData = frame.into();

        // sounds[5] should be in sounds[1][1] (vec4_index=1, element_index=1)
        assert_eq!(shader_data.sounds[1][1], 42);
        // ttls[10] should be in ttls[2][2] (vec4_index=2, element_index=2)
        assert_eq!(shader_data.ttls[2][2], 3.14);
        // notes[15] should be in notes[3][3] (vec4_index=3, element_index=3)
        assert_eq!(shader_data.notes[3][3], 440.0);
        // gains[7] should be in gains[1][3] (vec4_index=1, element_index=3)
        assert_eq!(shader_data.gains[1][3], 0.8);
    }

    #[test]
    fn test_osc_history_data_creation() {
        let history = OscHistoryData::new();
        assert_eq!(history.current_frame.sounds[0], 0);
        assert_eq!(history.ring_buffer.occupied_len(), 0);
    }

    #[test]
    fn test_osc_history_data_update_methods() {
        let mut history = OscHistoryData::new();

        history.update_sound(5, 100);
        history.update_ttl(10, 2.5);
        history.update_note(15, 880.0);
        history.update_gain(7, 0.9);

        assert_eq!(history.current_frame.sounds[5], 100);
        assert_eq!(history.current_frame.ttls[10], 2.5);
        assert_eq!(history.current_frame.notes[15], 880.0);
        assert_eq!(history.current_frame.gains[7], 0.9);
    }

    #[test]
    fn test_osc_history_data_ring_buffer_operations() {
        let mut history = OscHistoryData::new();

        // Set some test data
        history.update_sound(0, 1);
        history.push_current_frame();
        assert_eq!(history.ring_buffer.occupied_len(), 1);

        history.update_sound(0, 2);
        history.push_current_frame();
        assert_eq!(history.ring_buffer.occupied_len(), 2);
    }

    #[test]
    fn test_osc_history_data_ring_buffer_overwrite() {
        let mut history = OscHistoryData::new();

        // Fill beyond capacity to test overwrite behavior
        for i in 0..600 {
            history.update_sound(0, i);
            history.push_current_frame();
        }

        // Should not exceed HISTORY_SIZE
        assert_eq!(history.ring_buffer.occupied_len(), HISTORY_SIZE);
    }

    #[test]
    fn test_osc_history_data_prepare_shader_data() {
        let mut history = OscHistoryData::new();

        // Add some frames to history
        history.update_sound(0, 10);
        history.push_current_frame();

        history.update_sound(0, 20);
        // Don't push current frame to test current frame inclusion

        let shader_data = history.prepare_shader_data();

        // Should always return exactly HISTORY_SIZE frames
        assert_eq!(shader_data.len(), HISTORY_SIZE);

        // First frame should be current frame
        assert_eq!(shader_data[0].sounds[0][0], 20);

        // Second frame should be from ring buffer
        assert_eq!(shader_data[1].sounds[0][0], 10);

        // Remaining frames should be default (0)
        #[allow(clippy::needless_range_loop)]
        for i in 2..HISTORY_SIZE {
            assert_eq!(shader_data[i].sounds[0][0], 0);
        }
    }

    #[test]
    fn test_osc_history_bounds_checking() {
        let mut history = OscHistoryData::new();

        // Test bounds checking - should not panic
        history.update_sound(16, 999); // Out of bounds
        history.update_ttl(16, 999.0);
        history.update_note(16, 999.0);
        history.update_gain(16, 999.0);

        // Values should remain unchanged
        assert_eq!(history.current_frame.sounds[15], 0);
        assert_eq!(history.current_frame.ttls[15], 0.0);
        assert_eq!(history.current_frame.notes[15], 0.0);
        assert_eq!(history.current_frame.gains[15], 0.0);
    }

    #[test]
    fn test_osc_message_processing() {
        use rosc::{OscMessage, OscType};
        use std::collections::HashMap;

        // Create a mock OscInputManager without actual network setup
        let history_data = OscHistoryData::new();
        let mut sound_map = HashMap::new();
        sound_map.insert("bd", 1);
        sound_map.insert("sd", 2);

        let mut manager = OscInputManagerCore {
            history_data,
            sound_map,
        };

        // Create test OSC message similar to TidalCycles output
        let msg = OscMessage {
            addr: "/dirt/play".to_string(),
            args: vec![
                OscType::String("orbit".to_string()),
                OscType::Int(0),
                OscType::String("sound".to_string()),
                OscType::String("bd".to_string()),
                OscType::String("gain".to_string()),
                OscType::Float(0.8),
                OscType::String("delta".to_string()),
                OscType::Float(0.5),
                OscType::String("note".to_string()),
                OscType::Float(60.0),
            ],
        };

        manager.process_osc_message(&msg);

        // Check that values were set correctly
        assert_eq!(manager.history_data.current_frame.sounds[0], 1); // bd sound ID
        assert_eq!(manager.history_data.current_frame.gains[0], 0.8);
        assert_eq!(manager.history_data.current_frame.ttls[0], 0.5);
        assert_eq!(manager.history_data.current_frame.notes[0], 60.0);
    }

    // Helper struct for testing without full OscInputManager
    struct OscInputManagerCore {
        history_data: OscHistoryData,
        sound_map: HashMap<&'static str, i32>,
    }

    impl OscInputManagerCore {
        fn process_osc_message(&mut self, msg: &OscMessage) {
            // Same logic as in OscInputManager
            let mut id: usize = 0;
            let mut ttl = 0.0;
            let mut note = 0.0;
            let mut gain = 0.0;
            let mut sound = 0;

            for (i, v) in msg.args.iter().enumerate() {
                if let OscType::String(val) = v {
                    match val.as_str() {
                        "orbit" => {
                            if let Some(OscType::Int(orbit)) = msg.args.get(i + 1) {
                                id = *orbit as usize;
                            }
                        }
                        "delta" => {
                            if let Some(OscType::Float(delta)) = msg.args.get(i + 1) {
                                ttl = *delta;
                            }
                        }
                        "note" | "n" => {
                            if let Some(OscType::Float(n)) = msg.args.get(i + 1) {
                                note = *n;
                            }
                        }
                        "gain" => {
                            if let Some(OscType::Float(g)) = msg.args.get(i + 1) {
                                gain = *g;
                            }
                        }
                        "sound" | "s" => {
                            if let Some(OscType::String(s)) = msg.args.get(i + 1) {
                                if let Some(&sound_id) = self.sound_map.get(s.as_str()) {
                                    sound = sound_id;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if id < 16 {
                self.history_data.update_sound(id, sound);
                self.history_data.update_ttl(id, ttl);
                self.history_data.update_note(id, note);
                self.history_data.update_gain(id, gain);
            }
        }
    }
}

// ============================================================================
// Bevy Integration
// ============================================================================

use bevy::prelude::*;

/// OSC input manager with Bevy resource support (lifetime-free version)
#[derive(Resource)]
pub struct OscInputManager {
    pub(crate) history_data: OscHistoryData,
    pub buffer_handle: Handle<bevy::render::storage::ShaderStorageBuffer>,
    pub buffer_needs_update: bool,
    pub enabled: bool,
    sound_map: HashMap<String, i32>,
    receiver: Option<Receiver<OscPacket>>,
}

impl OscInputManager {
    pub const STORAGE_BINDING_INDEX: u32 = 0;

    pub fn new(
        buffer_handle: Handle<bevy::render::storage::ShaderStorageBuffer>,
        config: &crate::config::OscConfig,
    ) -> Self {
        let history_data = OscHistoryData::new();

        let mut sound_map = HashMap::new();
        for s in &config.sound {
            sound_map.insert(s.name.clone(), s.id);
        }

        // Start OSC server
        let port = config.port;
        let receiver = Some(async_std::task::block_on(osc_start(port)));
        log::info!("OSC server started on port {}", port);

        Self {
            history_data,
            buffer_handle,
            buffer_needs_update: true,
            enabled: true, // Always enabled if config exists
            sound_map,
            receiver,
        }
    }

    pub fn update(&mut self, _queue: Option<&wgpu::Queue>) {
        // Process incoming OSC messages
        if let Some(ref receiver) = self.receiver {
            match receiver.try_recv() {
                Ok(packet) => {
                    match packet {
                        OscPacket::Message(msg) => {
                            self.process_osc_message(&msg);
                        }
                        OscPacket::Bundle(bundle) => {
                            // Process first message in bundle (matching original behavior)
                            if let Some(OscPacket::Message(msg)) = bundle.content.first() {
                                self.process_osc_message(msg);
                            }
                        }
                    }
                }
                Err(_) => {
                    // No new messages, apply time decay
                    self.elapse_time();
                }
            }
        } else {
            // No receiver, just apply time decay
            self.elapse_time();
        }

        // Push current frame to history
        self.history_data.push_current_frame();
        self.buffer_needs_update = true;
    }

    fn process_osc_message(&mut self, msg: &OscMessage) {
        // Parse OSC message parameters
        let mut id: usize = 0;
        let mut ttl = 0.0;
        let mut note = 0.0;
        let mut gain = 0.0;
        let mut sound = 0;

        for (i, v) in msg.args.iter().enumerate() {
            if let OscType::String(val) = v {
                match val.as_str() {
                    "orbit" => {
                        if let Some(OscType::Int(orbit)) = msg.args.get(i + 1) {
                            id = *orbit as usize;
                        }
                    }
                    "delta" => {
                        if let Some(OscType::Float(delta)) = msg.args.get(i + 1) {
                            ttl = *delta;
                        }
                    }
                    "note" | "n" => {
                        if let Some(OscType::Float(n)) = msg.args.get(i + 1) {
                            note = *n;
                        }
                    }
                    "gain" => {
                        if let Some(OscType::Float(g)) = msg.args.get(i + 1) {
                            gain = *g;
                        }
                    }
                    "sound" | "s" => {
                        if let Some(OscType::String(s)) = msg.args.get(i + 1) {
                            if let Some(&sound_id) = self.sound_map.get(s.as_str()) {
                                sound = sound_id;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Update values if id is within bounds
        if id < 16 {
            self.history_data.update_sound(id, sound);
            self.history_data.update_ttl(id, ttl);
            self.history_data.update_note(id, note);
            self.history_data.update_gain(id, gain);
        }
    }

    fn elapse_time(&mut self) {
        // Apply time decay to TTL values and clear expired entries
        // Note: We don't have time_delta here, so we'll use a fixed small decay
        let time_delta = 1.0 / 60.0; // Assuming 60 FPS

        for i in 0..16 {
            let current_ttl = self.history_data.current_frame.ttls[i];
            let new_ttl = (current_ttl - time_delta).max(0.0);

            if new_ttl <= 0.0 {
                // Clear expired entry
                self.history_data.update_sound(i, 0);
                self.history_data.update_ttl(i, 0.0);
                self.history_data.update_note(i, 0.0);
                self.history_data.update_gain(i, 0.0);
            } else {
                // Update TTL
                self.history_data.update_ttl(i, new_ttl);
            }
        }
    }

    pub fn write_buffer(
        &self,
        storage_buffers: &mut ResMut<Assets<bevy::render::storage::ShaderStorageBuffer>>,
    ) {
        let shader_data = self.history_data.prepare_shader_data();
        let data_bytes = bytemuck::cast_slice(&shader_data);

        if let Some(buffer) = storage_buffers.get_mut(&self.buffer_handle) {
            buffer.data = Some(data_bytes.to_vec());
        }
    }

    pub fn get_shader_data(&self) -> Vec<OscShaderData> {
        self.history_data.prepare_shader_data()
    }
}

/// Bevy system for updating OSC input
pub fn osc_input_system(
    mut osc_manager: Option<ResMut<OscInputManager>>,
    mut storage_buffers: ResMut<Assets<bevy::render::storage::ShaderStorageBuffer>>,
) {
    if let Some(ref mut manager) = osc_manager {
        manager.update(None);

        // Write updated data to storage buffer
        if manager.buffer_needs_update {
            manager.write_buffer(&mut storage_buffers);
            manager.buffer_needs_update = false;
        }
    }
}

/// Bevy startup system for initializing OSC input
pub fn setup_osc_input_system(
    mut commands: Commands,
    config: Res<crate::ShekereConfig>,
    buffer_handles: Option<Res<crate::shader_renderer::InputBufferHandles>>,
) {
    if let Some(osc_config) = &config.config.osc {
        log::info!("Setting up OSC input system");

        if let Some(handles) = buffer_handles {
            let osc_manager = OscInputManager::new(handles.osc_buffer.clone(), osc_config);
            commands.insert_resource(osc_manager);
            log::info!("OSC input system setup completed with ShaderStorageBuffer");
        } else {
            log::warn!("InputBufferHandles not available, OSC input will not work");
        }
    }
}
