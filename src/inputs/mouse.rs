#[cfg(test)]
use ringbuf::traits::Observer;
use ringbuf::{
    HeapRb,
    traits::{Consumer, RingBuffer},
};
use std::sync::{Arc, Mutex};

// Individual frame data for ring buffer storage
#[derive(Debug, Clone, Copy)]
pub struct MouseFrameData {
    position: [f32; 2],
}

impl MouseFrameData {
    pub fn new(x: f32, y: f32) -> Self {
        Self { position: [x, y] }
    }
}

// GPU-friendly data format with vec4 alignment
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MouseShaderData {
    position: [f32; 2],
    _padding: [f32; 2], // vec4 alignment for GPU efficiency
}

impl MouseShaderData {
    fn from_frame_data(frame_data: &MouseFrameData) -> Self {
        Self {
            position: frame_data.position,
            _padding: [0.0, 0.0],
        }
    }
}

// History data structure using ring buffer only (optimized)
pub struct MouseHistoryData {
    pub current_frame: MouseFrameData,
    ring_buffer: HeapRb<MouseFrameData>,
}

impl MouseHistoryData {
    pub(crate) fn new() -> Self {
        Self {
            current_frame: MouseFrameData::new(0.0, 0.0),
            ring_buffer: HeapRb::new(512),
        }
    }

    pub(crate) fn push_current_frame(&mut self) {
        self.ring_buffer.push_overwrite(self.current_frame);
    }

    pub fn prepare_shader_data(&self) -> Vec<MouseShaderData> {
        let mut shader_data = Vec::with_capacity(512);

        // Add current frame first (index 0)
        shader_data.push(MouseShaderData::from_frame_data(&self.current_frame));

        // Add history frames in reverse order (newest to oldest)
        // Limit to 511 frames to ensure total is exactly 512
        for frame in self.ring_buffer.iter().rev() {
            if shader_data.len() >= 512 {
                break;
            }
            shader_data.push(MouseShaderData::from_frame_data(frame));
        }

        // Fill remaining slots with default data
        while shader_data.len() < 512 {
            shader_data.push(MouseShaderData::from_frame_data(&MouseFrameData::new(
                0.0, 0.0,
            )));
        }

        shader_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // MouseFrameData tests
    #[test]
    fn test_mouse_frame_data_creation() {
        let frame = MouseFrameData::new(100.0, 200.0);
        assert_eq!(frame.position, [100.0, 200.0]);
    }

    #[test]
    fn test_mouse_frame_data_from_values() {
        let frame = MouseFrameData::new(150.0, 250.0);
        assert_eq!(frame.position, [150.0, 250.0]);
    }

    #[test]
    fn test_mouse_frame_data_debug_clone_copy() {
        let frame1 = MouseFrameData::new(10.0, 20.0);
        let frame2 = frame1; // Copy
        let frame3 = frame1; // Copy (no need to clone for Copy types)

        assert_eq!(frame1.position, frame2.position);
        assert_eq!(frame1.position, frame3.position);

        // Test Debug formatting
        let debug_str = format!("{:?}", frame1);
        assert!(debug_str.contains("MouseFrameData"));
    }

    // MouseShaderData tests
    #[test]
    fn test_mouse_shader_data_from_frame_data() {
        let frame = MouseFrameData::new(100.0, 200.0);
        let shader_data = MouseShaderData::from_frame_data(&frame);
        assert_eq!(shader_data.position, [100.0, 200.0]);
        assert_eq!(shader_data._padding, [0.0, 0.0]);
    }

    #[test]
    fn test_mouse_shader_data_gpu_alignment() {
        use std::mem;

        // Verify struct is 16 bytes (4 f32 values) for vec4 alignment
        assert_eq!(mem::size_of::<MouseShaderData>(), 16);
        assert_eq!(mem::align_of::<MouseShaderData>(), 4);
    }

    #[test]
    fn test_mouse_shader_data_bytemuck() {
        let frame = MouseFrameData::new(123.45, 678.90);
        let shader_data = MouseShaderData::from_frame_data(&frame);
        let data_array = [shader_data];
        let bytes: &[u8] = bytemuck::cast_slice(&data_array);
        assert_eq!(bytes.len(), 16);

        let reconstructed: MouseShaderData = bytemuck::cast_slice::<u8, MouseShaderData>(bytes)[0];
        assert_eq!(reconstructed.position, [123.45, 678.90]);
        assert_eq!(reconstructed._padding, [0.0, 0.0]);
    }

    // MouseHistoryData tests
    #[test]
    fn test_mouse_history_data_creation() {
        let history = MouseHistoryData::new();

        // Verify initial state
        assert_eq!(history.current_frame.position, [0.0, 0.0]);

        // Verify ring buffer is empty initially
        assert_eq!(history.ring_buffer.occupied_len(), 0);
    }

    #[test]
    fn test_mouse_history_data_push_current_frame() {
        let mut history = MouseHistoryData::new();

        // Push first frame
        history.current_frame = MouseFrameData::new(10.0, 20.0);
        history.push_current_frame();
        assert_eq!(history.ring_buffer.occupied_len(), 1);

        // Push second frame
        history.current_frame = MouseFrameData::new(30.0, 40.0);
        history.push_current_frame();
        assert_eq!(history.ring_buffer.occupied_len(), 2);
    }

    #[test]
    fn test_mouse_history_data_ring_buffer_overwrite() {
        let mut history = MouseHistoryData::new();

        // Fill ring buffer to capacity (512 frames)
        for i in 0..512 {
            history.current_frame = MouseFrameData::new(i as f32, (i * 2) as f32);
            history.push_current_frame();
        }
        assert_eq!(history.ring_buffer.occupied_len(), 512);

        // Add one more frame - should overwrite oldest
        history.current_frame = MouseFrameData::new(1000.0, 2000.0);
        history.push_current_frame();
        assert_eq!(history.ring_buffer.occupied_len(), 512); // Still at capacity
    }

    #[test]
    fn test_mouse_history_data_prepare_shader_data() {
        let mut history = MouseHistoryData::new();

        // Add some test frames to history (but not current)
        for i in 0..4 {
            history.current_frame = MouseFrameData::new(i as f32 * 10.0, i as f32 * 20.0);
            history.push_current_frame();
        }

        // Set current frame (not pushed to history yet)
        history.current_frame = MouseFrameData::new(40.0, 80.0);

        // Prepare shader data
        let shader_data = history.prepare_shader_data();

        // Verify shader data array has 512 elements
        assert_eq!(shader_data.len(), 512);

        // Verify current frame is at index 0
        assert_eq!(shader_data[0].position, [40.0, 80.0]); // Current frame

        // Verify history frames are in correct order (newest to oldest)
        assert_eq!(shader_data[1].position, [30.0, 60.0]); // Most recent in history
        assert_eq!(shader_data[2].position, [20.0, 40.0]); // 2nd most recent
        assert_eq!(shader_data[3].position, [10.0, 20.0]); // 3rd most recent
        assert_eq!(shader_data[4].position, [0.0, 0.0]); // Oldest in history
    }

    // MouseInputManager tests
    #[test]
    fn test_mouse_input_manager_creation() {
        use bevy::render::storage::ShaderStorageBuffer;

        // Create a mock buffer handle for testing
        let buffer_handle = Handle::<ShaderStorageBuffer>::default();
        let manager = MouseInputManager::new(buffer_handle);

        // Verify history data is initialized
        if let Ok(history) = manager.history_data.lock() {
            assert_eq!(history.current_frame.position, [0.0, 0.0]);
            assert_eq!(history.ring_buffer.occupied_len(), 0);
        }
    }

    #[test]
    fn test_mouse_input_manager_update() {
        use bevy::render::storage::ShaderStorageBuffer;

        // Create a mock buffer handle for testing
        let buffer_handle = Handle::<ShaderStorageBuffer>::default();
        let mut manager = MouseInputManager::new(buffer_handle);

        let position = bevy::math::Vec2::new(100.0, 200.0);
        manager.update_position(position);

        // Check that current frame was updated
        if let Ok(history) = manager.history_data.lock() {
            assert_eq!(history.current_frame.position, [100.0, 200.0]);
            // Should have pushed previous frame (which was 0,0) to history
            assert_eq!(history.ring_buffer.occupied_len(), 1);
        }
    }

    // Helper function to create a test device
    #[allow(dead_code)] // Kept for future GPU-related tests
    fn create_test_device() -> wgpu::Device {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            env_logger::init();
        });

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            }))
            .unwrap();

        let (device, _queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: Default::default(),
            },
            None,
        ))
        .unwrap();

        device
    }
}

// ============================================================================
// Bevy Integration
// ============================================================================

use bevy::prelude::*;
use bevy::window::CursorMoved;

/// Mouse input manager with Bevy resource support
#[derive(Resource)]
pub struct MouseInputManager {
    pub history_data: Arc<Mutex<MouseHistoryData>>,
    pub buffer_handle: Handle<bevy::render::storage::ShaderStorageBuffer>,
    pub buffer_needs_update: bool,
}

impl MouseInputManager {
    pub const BINDING_INDEX: u32 = 0;

    pub fn new(buffer_handle: Handle<bevy::render::storage::ShaderStorageBuffer>) -> Self {
        let history_data = Arc::new(Mutex::new(MouseHistoryData::new()));

        Self {
            history_data,
            buffer_handle,
            buffer_needs_update: true,
        }
    }

    pub fn update_position(&mut self, position: Vec2) {
        // Update history data
        if let Ok(mut history) = self.history_data.lock() {
            // Push the previous frame to history
            history.push_current_frame();
            // Update current frame with new position
            history.current_frame = MouseFrameData::new(position.x, position.y);
            self.buffer_needs_update = true;
        }
    }

    pub fn update(&mut self) {
        // Mouse update only happens when position changes
        if self.buffer_needs_update {
            self.buffer_needs_update = false;
        }
    }

    pub fn write_buffer(
        &self,
        storage_buffers: &mut ResMut<Assets<bevy::render::storage::ShaderStorageBuffer>>,
    ) {
        // Write storage buffer with history data
        if let Ok(history) = self.history_data.lock() {
            let shader_data = history.prepare_shader_data();
            let data_bytes = bytemuck::cast_slice(&shader_data);

            if let Some(buffer) = storage_buffers.get_mut(&self.buffer_handle) {
                buffer.data = Some(data_bytes.to_vec());
            }
        }
    }

    pub fn get_shader_data(&self) -> Vec<MouseShaderData> {
        self.history_data.lock().unwrap().prepare_shader_data()
    }
}

/// Bevy system for updating mouse input
pub fn mouse_input_system(
    mut cursor_moved_events: MessageReader<CursorMoved>,
    mut mouse_manager: Option<ResMut<MouseInputManager>>,
    mut storage_buffers: ResMut<Assets<bevy::render::storage::ShaderStorageBuffer>>,
) {
    if let Some(ref mut manager) = mouse_manager {
        for event in cursor_moved_events.read() {
            manager.update_position(event.position);
        }

        // Write updated data to storage buffer
        if manager.buffer_needs_update {
            manager.write_buffer(&mut storage_buffers);
            manager.buffer_needs_update = false;
        }
    }
}

/// Bevy startup system for initializing mouse input
pub fn setup_mouse_input_system(
    mut commands: Commands,
    buffer_handles: Option<Res<crate::shader_renderer::InputBufferHandles>>,
) {
    log::info!("Setting up Mouse input system");

    if let Some(handles) = buffer_handles {
        let mouse_manager = MouseInputManager::new(handles.mouse_buffer.clone());
        commands.insert_resource(mouse_manager);
        log::info!("Mouse input system setup completed with ShaderStorageBuffer");
    } else {
        log::warn!("InputBufferHandles not available, mouse input will not work");
    }
}
