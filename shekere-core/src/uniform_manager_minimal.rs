/// Enhanced UniformManager for WASM targets (Phase 2)
/// This version supports full IPC-driven uniform management including audio, MIDI, and OSC data
/// received from the native backend via Tauri IPC

use crate::bind_group_factory::BindGroupFactory;
use crate::config::Config;
use crate::inputs::mouse::MouseInputManager;
use crate::uniforms::time_uniform::TimeUniform;
use crate::uniforms::window_uniform::WindowUniform;
use bytemuck;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UniformManagerError {
    #[error("Failed to create bind group: {0}")]
    BindGroupCreation(String),
}

/// Enhanced UniformManager for WASM targets with IPC support
pub struct UniformManager<'a> {
    // Core uniforms
    pub window_uniform: WindowUniform,
    pub time_uniform: TimeUniform,

    // Input managers (mouse only for WASM)
    pub mouse_input_manager: Option<MouseInputManager>,

    // IPC data buffers (for audio/MIDI/OSC data received from backend)
    pub spectrum_buffer: Option<wgpu::Buffer>,
    pub midi_buffer: Option<wgpu::Buffer>,
    pub osc_buffer: Option<wgpu::Buffer>,

    // Bind groups
    pub uniform_bind_group: wgpu::BindGroup,
    pub device_bind_group: wgpu::BindGroup,
    pub sound_bind_group: Option<wgpu::BindGroup>,

    // Bind group layouts (stored for pipeline creation)
    pub uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub device_bind_group_layout: wgpu::BindGroupLayout,
    pub sound_bind_group_layout: Option<wgpu::BindGroupLayout>,

    // IPC-driven data state
    current_spectrum_data: Option<crate::ipc_protocol::SpectrumData>,
    current_midi_data: Option<crate::ipc_protocol::MidiData>,
    current_osc_data: Option<crate::ipc_protocol::OscData>,

    // Keep lifetime parameter for compatibility
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> UniformManager<'a> {
    /// Create an enhanced UniformManager for WASM with IPC support
    pub async fn new(
        device: &wgpu::Device,
        _config: &'a Config,
        window_width: u32,
        window_height: u32,
    ) -> Result<Self, UniformManagerError> {
        // Create uniforms
        let window_uniform = WindowUniform::new_with_size(device, window_width, window_height);
        let time_uniform = TimeUniform::new(device);

        // Create minimal input managers (mouse only)
        let mouse_input_manager = Some(MouseInputManager::new(device));

        // Create IPC data buffers (initially empty, will be populated via IPC)
        let spectrum_buffer = Some(Self::create_spectrum_buffer(device));
        let midi_buffer = Some(Self::create_midi_buffer(device));
        let osc_buffer = Some(Self::create_osc_buffer(device));

        // Create bind groups using BindGroupFactory
        let mut uniform_factory = BindGroupFactory::new();

        // Add uniform buffers (time + window)
        uniform_factory.add_entry(0, &time_uniform.buffer);
        uniform_factory.add_entry(1, &window_uniform.buffer);

        let (uniform_bind_group_layout, uniform_bind_group) = uniform_factory.create(device, "uniform");
        let uniform_bind_group_layout = uniform_bind_group_layout.expect("Failed to create uniform bind group layout");
        let uniform_bind_group = uniform_bind_group.expect("Failed to create uniform bind group");

        // Create device bind group (mouse)
        let mut device_factory = BindGroupFactory::new();

        if let Some(ref mim) = mouse_input_manager {
            device_factory.add_storage_entry(0, &mim.buffer);
        }

        let (device_bind_group_layout, device_bind_group) = if device_factory.entries.is_empty() {
            // Create empty bind group layout and group
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[],
                label: Some("empty_device_bind_group_layout"),
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[],
                label: Some("empty_device_bind_group"),
            });
            (layout, group)
        } else {
            let (layout, group) = device_factory.create(device, "device");
            (layout.expect("Failed to create device bind group layout"),
             group.expect("Failed to create device bind group"))
        };

        // Create sound bind group for IPC data (spectrum, MIDI, OSC)
        let mut sound_factory = BindGroupFactory::new();

        if let Some(ref spectrum_buf) = spectrum_buffer {
            sound_factory.add_storage_entry(1, spectrum_buf); // Binding 1 for spectrum
        }
        if let Some(ref midi_buf) = midi_buffer {
            sound_factory.add_storage_entry(2, midi_buf); // Binding 2 for MIDI
        }
        if let Some(ref osc_buf) = osc_buffer {
            sound_factory.add_storage_entry(3, osc_buf); // Binding 3 for OSC
        }

        let (sound_bind_group_layout, sound_bind_group) = if sound_factory.entries.is_empty() {
            (None, None)
        } else {
            let (layout, group) = sound_factory.create(device, "sound");
            (layout, group)
        };

        Ok(Self {
            window_uniform,
            time_uniform,
            mouse_input_manager,
            spectrum_buffer,
            midi_buffer,
            osc_buffer,
            uniform_bind_group,
            device_bind_group,
            sound_bind_group,
            uniform_bind_group_layout,
            device_bind_group_layout,
            sound_bind_group_layout,
            current_spectrum_data: None,
            current_midi_data: None,
            current_osc_data: None,
            _marker: std::marker::PhantomData,
        })
    }

    /// Create a spectrum buffer for IPC data
    fn create_spectrum_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        // Create buffer for spectrum data (512 f32 values + metadata)
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spectrum_buffer"),
            size: (512 * std::mem::size_of::<f32>() + 16) as u64, // 512 frequencies + metadata
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Create a MIDI buffer for IPC data
    fn create_midi_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        // Create buffer for MIDI data (128 notes + controls + metadata)
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("midi_buffer"),
            size: (128 * 4 + 128 * 4 + 64) as u64, // note velocities + controls + metadata
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Create an OSC buffer for IPC data
    fn create_osc_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        // Create buffer for OSC data (64 f32 values for various OSC parameters)
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("osc_buffer"),
            size: (64 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Update all uniforms and input data
    pub fn update(&mut self, queue: &wgpu::Queue, delta_time: f32) {
        // Update time uniform
        self.time_uniform.update(delta_time);
        self.time_uniform.write_buffer(queue);

        // Update window uniform (in case size changed)
        self.window_uniform.write_buffer(queue);

        // Update mouse input if available
        if let Some(ref mut mouse_manager) = self.mouse_input_manager {
            mouse_manager.write_buffer(queue);
        }

        // Update IPC data buffers if we have new data
        self.update_ipc_buffers(queue);
    }

    /// Update IPC data buffers with current data
    fn update_ipc_buffers(&self, queue: &wgpu::Queue) {
        // Update spectrum buffer
        if let (Some(buffer), Some(data)) = (&self.spectrum_buffer, &self.current_spectrum_data) {
            let mut spectrum_data = Vec::with_capacity(512 + 4);
            
            // Add frequency data (pad or truncate to 512)
            if data.frequencies.len() >= 512 {
                spectrum_data.extend_from_slice(&data.frequencies[0..512]);
            } else {
                spectrum_data.extend_from_slice(&data.frequencies);
                spectrum_data.resize(512, 0.0); // Pad with zeros
            }
            
            // Add metadata
            spectrum_data.push(data.amplitude);
            spectrum_data.push(data.peak_frequency);
            spectrum_data.push(data.sample_rate as f32);
            spectrum_data.push(data.frequencies.len() as f32); // Original length
            
            let data_bytes = bytemuck::cast_slice(&spectrum_data);
            queue.write_buffer(buffer, 0, data_bytes);
        }

        // Update MIDI buffer
        if let (Some(buffer), Some(data)) = (&self.midi_buffer, &self.current_midi_data) {
            let mut midi_data = vec![0.0f32; 128 + 128 + 16]; // notes + controls + metadata

            // Copy active notes into velocity array
            for &(note, velocity) in &data.active_notes {
                if note < 128 {
                    midi_data[note as usize] = velocity as f32 / 127.0; // Normalize to 0.0-1.0
                }
            }

            // Copy control changes
            for &(controller, value) in &data.control_changes {
                if controller < 128 {
                    midi_data[128 + controller as usize] = value as f32 / 127.0; // Normalize to 0.0-1.0
                }
            }

            // Add metadata (starting at index 256)
            midi_data[256] = data.program_change.unwrap_or(0) as f32;
            midi_data[257] = data.pitch_bend as f32;

            let data_bytes = bytemuck::cast_slice(&midi_data);
            queue.write_buffer(buffer, 0, data_bytes);
        }

        // Update OSC buffer
        if let (Some(buffer), Some(data)) = (&self.osc_buffer, &self.current_osc_data) {
            let mut osc_data = vec![0.0f32; 64];
            
            // Convert OSC messages to float array
            for (i, message) in data.messages.iter().enumerate() {
                if i >= 16 { break; } // Limit to 16 messages
                
                let base_idx = i * 4;
                if base_idx + 3 < osc_data.len() {
                    // Store first argument if available
                    if let Some(arg) = message.args.first() {
                        match arg {
                            crate::ipc_protocol::OscArg::Float(f) => osc_data[base_idx] = *f,
                            crate::ipc_protocol::OscArg::Int(i) => osc_data[base_idx] = *i as f32,
                            crate::ipc_protocol::OscArg::String(_) => osc_data[base_idx] = 1.0,
                            crate::ipc_protocol::OscArg::Bool(b) => osc_data[base_idx] = if *b { 1.0 } else { 0.0 },
                            crate::ipc_protocol::OscArg::Blob(_) => osc_data[base_idx] = 0.0, // Default for blob
                        }
                    }
                    // Store address hash as identifier
                    osc_data[base_idx + 1] = message.address.len() as f32;
                }
            }
            
            let data_bytes = bytemuck::cast_slice(&osc_data);
            queue.write_buffer(buffer, 0, data_bytes);
        }
    }

    /// Handle IPC uniform data update
    pub fn handle_ipc_uniform_data(&mut self, uniform_data: crate::ipc_protocol::UniformData) {
        // Update internal data state
        self.current_spectrum_data = uniform_data.spectrum;
        self.current_midi_data = uniform_data.midi;
        self.current_osc_data = uniform_data.osc;

        // Update window resolution if it changed
        let new_width = uniform_data.resolution[0] as u32;
        let new_height = uniform_data.resolution[1] as u32;
        if new_width != self.window_uniform.data.width() as u32 ||
           new_height != self.window_uniform.data.height() as u32 {
            self.window_uniform.update_size(new_width, new_height);
        }

        // Update mouse data if available
        if let Some(mouse_data) = uniform_data.mouse {
            if let Some(mouse_manager) = &mut self.mouse_input_manager {
                // Convert normalized coordinates to pixel coordinates
                let pixel_x = (mouse_data.position[0] * new_width as f32) as f64;
                let pixel_y = (mouse_data.position[1] * new_height as f32) as f64;
                mouse_manager.update(pixel_x, pixel_y);

                // Handle button states if needed
                // TODO: Extend MouseInputManager to support button states
            }
        }
    }

    /// Update window size
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_uniform.update_size(width, height);
    }

    /// Get uniform bind group layout
    pub fn uniform_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.uniform_bind_group_layout
    }

    /// Get device bind group layout
    pub fn device_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.device_bind_group_layout
    }

    /// Get uniform bind group
    pub fn uniform_bind_group(&self) -> &wgpu::BindGroup {
        &self.uniform_bind_group
    }

    /// Get device bind group
    pub fn device_bind_group(&self) -> &wgpu::BindGroup {
        &self.device_bind_group
    }

    /// Get bind group layouts (for renderer compatibility)
    pub fn get_bind_group_layouts(&self) -> Vec<&wgpu::BindGroupLayout> {
        let mut layouts = vec![&self.uniform_bind_group_layout, &self.device_bind_group_layout];
        if let Some(layout) = &self.sound_bind_group_layout {
            layouts.push(layout);
        }
        layouts
    }

    /// Handle mouse input (for renderer compatibility)
    pub fn handle_mouse_input(&mut self, x: f64, y: f64) {
        if let Some(ref mut mouse_manager) = self.mouse_input_manager {
            mouse_manager.update(x, y);
        }
    }
}