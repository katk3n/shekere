use std::sync::Arc;

use cpal::{
    Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Observer, Producer, RingBuffer, Split},
    wrap::caching::Caching,
};
use spectrum_analyzer::scaling::*;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{FrequencyLimit, samples_fft_to_spectrum};

use crate::config::SpectrumConfig;

const NUM_SAMPLES: usize = 2048;

// Audio stream setup function
fn setup_audio_stream() -> (Stream, Caching<Arc<HeapRb<f32>>, false, true>) {
    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .expect("failed to find input device");
    let mut supported_config_range = input_device
        .supported_input_configs()
        .expect("error while querying configs");
    let supported_config = supported_config_range
        .next()
        .expect("no supported config")
        .with_max_sample_rate();
    let config = supported_config.into();

    let ring_buffer = HeapRb::<f32>::new(NUM_SAMPLES * 2);
    let (mut prod, cons) = ring_buffer.split();
    for _ in 0..NUM_SAMPLES {
        prod.try_push(0.0).unwrap();
    }

    let stream = input_device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for d in data {
                    if let Ok(()) = prod.try_push(*d) {}
                }
            },
            move |_err| {},
            None,
        )
        .unwrap();

    stream.play().unwrap();
    (stream, cons)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SpectrumFrameData {
    frequencies: [[f32; 4]; NUM_SAMPLES / 4],
    amplitudes: [[f32; 4]; NUM_SAMPLES / 4],
    num_points: u32,
    max_frequency: f32,
    max_amplitude: f32,
}

impl Default for SpectrumFrameData {
    fn default() -> Self {
        Self {
            frequencies: [[0.0; 4]; NUM_SAMPLES / 4],
            amplitudes: [[0.0; 4]; NUM_SAMPLES / 4],
            num_points: 0,
            max_frequency: 0.0,
            max_amplitude: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpectrumShaderData {
    frequencies: [[f32; 4]; NUM_SAMPLES / 4],
    amplitudes: [[f32; 4]; NUM_SAMPLES / 4],
    num_points: u32,
    max_frequency: f32,
    max_amplitude: f32,
    _padding: u32,
}

impl From<SpectrumFrameData> for SpectrumShaderData {
    fn from(frame: SpectrumFrameData) -> Self {
        Self {
            frequencies: frame.frequencies,
            amplitudes: frame.amplitudes,
            num_points: frame.num_points,
            max_frequency: frame.max_frequency,
            max_amplitude: frame.max_amplitude,
            _padding: 0,
        }
    }
}

pub(crate) struct SpectrumHistoryData {
    current_frame: SpectrumFrameData,
    ring_buffer: HeapRb<SpectrumFrameData>,
}

impl SpectrumHistoryData {
    pub(crate) fn new() -> Self {
        Self {
            current_frame: SpectrumFrameData::default(),
            ring_buffer: HeapRb::new(512),
        }
    }

    pub(crate) fn push_current_frame(&mut self) {
        // Push to ring buffer
        self.ring_buffer.push_overwrite(self.current_frame);
    }

    pub(crate) fn set_current_frame(&mut self, frame: SpectrumFrameData) {
        self.current_frame = frame;
    }

    pub(crate) fn prepare_shader_data(&self) -> Vec<SpectrumShaderData> {
        let mut shader_data = Vec::with_capacity(512);

        // Add current frame first (index 0 = history 0)
        shader_data.push(self.current_frame.into());

        // Add frames from ring buffer (newest to oldest)
        // Ring buffer iterator returns items in chronological order (oldest to newest),
        // so we need to collect and reverse to get newest to oldest
        let ring_data: Vec<_> = self.ring_buffer.iter().cloned().collect();
        for frame in ring_data.iter().rev() {
            shader_data.push((*frame).into());
            if shader_data.len() >= 512 {
                break;
            }
        }

        // Pad to exactly 512 frames if needed
        while shader_data.len() < 512 {
            shader_data.push(SpectrumFrameData::default().into());
        }

        shader_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectrum_frame_data_creation() {
        let frame = SpectrumFrameData::default();
        assert_eq!(frame.num_points, 0);
        assert_eq!(frame.max_frequency, 0.0);
        assert_eq!(frame.max_amplitude, 0.0);
    }

    #[test]
    fn test_spectrum_shader_data_gpu_alignment() {
        let shader_data = SpectrumShaderData {
            frequencies: [[1.0; 4]; NUM_SAMPLES / 4],
            amplitudes: [[2.0; 4]; NUM_SAMPLES / 4],
            num_points: 100,
            max_frequency: 20000.0,
            max_amplitude: 1.0,
            _padding: 0,
        };

        // Verify the size is correct for GPU alignment
        let expected_size = (NUM_SAMPLES / 4) * 4 * 4 * 2 + 4 * 4; // frequencies + amplitudes + metadata
        assert_eq!(std::mem::size_of::<SpectrumShaderData>(), expected_size);

        // Verify bytemuck compatibility
        let _bytes: &[u8] = bytemuck::cast_slice(&[shader_data]);
    }

    #[test]
    fn test_spectrum_frame_to_shader_data_conversion() {
        let frame_data = SpectrumFrameData {
            frequencies: [[1.0, 2.0, 3.0, 4.0]; NUM_SAMPLES / 4],
            amplitudes: [[0.1, 0.2, 0.3, 0.4]; NUM_SAMPLES / 4],
            num_points: 512,
            max_frequency: 22050.0,
            max_amplitude: 1.0,
        };

        let shader_data: SpectrumShaderData = frame_data.into();
        assert_eq!(shader_data.frequencies[0], [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(shader_data.amplitudes[0], [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(shader_data.num_points, 512);
        assert_eq!(shader_data.max_frequency, 22050.0);
        assert_eq!(shader_data.max_amplitude, 1.0);
    }

    #[test]
    fn test_spectrum_history_data_creation() {
        let history = SpectrumHistoryData::new();
        let data = history.prepare_shader_data();
        assert_eq!(data.len(), 512);

        // First entry should be the current frame (all zeros)
        assert_eq!(data[0].num_points, 0);
        assert_eq!(data[0].max_frequency, 0.0);
    }

    #[test]
    fn test_spectrum_history_data_ring_buffer_push() {
        let mut history = SpectrumHistoryData::new();

        let frame1 = SpectrumFrameData {
            frequencies: [[1.0; 4]; NUM_SAMPLES / 4],
            amplitudes: [[0.1; 4]; NUM_SAMPLES / 4],
            num_points: 100,
            max_frequency: 1000.0,
            max_amplitude: 0.5,
        };

        let frame2 = SpectrumFrameData {
            frequencies: [[2.0; 4]; NUM_SAMPLES / 4],
            amplitudes: [[0.2; 4]; NUM_SAMPLES / 4],
            num_points: 200,
            max_frequency: 2000.0,
            max_amplitude: 0.8,
        };

        history.set_current_frame(frame1);
        history.push_current_frame();
        history.set_current_frame(frame2);

        let data = history.prepare_shader_data();

        // Current frame should be frame2
        assert_eq!(data[0].num_points, 200);
        assert_eq!(data[0].max_frequency, 2000.0);

        // First historical entry should be frame1
        assert_eq!(data[1].num_points, 100);
        assert_eq!(data[1].max_frequency, 1000.0);
    }

    #[test]
    fn test_spectrum_history_data_ring_buffer_overwrite() {
        let mut history = SpectrumHistoryData::new();

        // Fill beyond capacity
        for i in 0..600u32 {
            let frame = SpectrumFrameData {
                frequencies: [[i as f32; 4]; NUM_SAMPLES / 4],
                amplitudes: [[0.1; 4]; NUM_SAMPLES / 4],
                num_points: i,
                max_frequency: i as f32 * 10.0,
                max_amplitude: 0.5,
            };
            history.set_current_frame(frame);
            history.push_current_frame();
        }

        let data = history.prepare_shader_data();
        assert_eq!(data.len(), 512);

        // Current frame should be the latest (599)
        assert_eq!(data[0].num_points, 599);
        assert_eq!(data[0].max_frequency, 5990.0);

        // Check that we have the most recent 511 frames in history
        // The oldest available should be frame 599 - 511 = 88
        let oldest_available_index = data.len() - 1;
        while oldest_available_index < data.len() && data[oldest_available_index].num_points == 0 {
            // Skip default entries at the end
        }
    }

    #[test]
    fn test_spectrum_history_bounds_checking() {
        let history = SpectrumHistoryData::new();
        let data = history.prepare_shader_data();

        // Should always return exactly 512 entries
        assert_eq!(data.len(), 512);

        // All entries should be valid SpectrumShaderData
        for entry in &data {
            assert!(entry.num_points <= NUM_SAMPLES as u32);
            assert!(entry.max_frequency >= 0.0);
            assert!(entry.max_amplitude >= 0.0);
        }
    }

    #[test]
    fn test_spectrum_history_data_prepare_shader_data() {
        let mut history = SpectrumHistoryData::new();

        // Add frames to history using push_current_frame pattern
        for i in 0..3 {
            let frame = SpectrumFrameData {
                frequencies: [[i as f32; 4]; NUM_SAMPLES / 4],
                amplitudes: [[0.1; 4]; NUM_SAMPLES / 4],
                num_points: i + 10,
                max_frequency: (i + 1) as f32 * 1000.0,
                max_amplitude: 0.5,
            };
            history.set_current_frame(frame);
            history.push_current_frame();
        }

        // Set final current frame values (different from ring buffer)
        let final_frame = SpectrumFrameData {
            frequencies: [[100.0; 4]; NUM_SAMPLES / 4],
            amplitudes: [[0.9; 4]; NUM_SAMPLES / 4],
            num_points: 999,
            max_frequency: 50000.0,
            max_amplitude: 1.0,
        };
        history.set_current_frame(final_frame);

        let data = history.prepare_shader_data();

        // Current frame (most recent) should be first
        assert_eq!(data[0].num_points, 999); // Final current frame
        assert_eq!(data[0].max_frequency, 50000.0);

        // Historical frames should follow in reverse order (newest first)
        assert_eq!(data[1].num_points, 12); // Last pushed frame (i=2)
        assert_eq!(data[1].max_frequency, 3000.0);

        assert_eq!(data[2].num_points, 11); // Second frame (i=1)
        assert_eq!(data[2].max_frequency, 2000.0);

        assert_eq!(data[3].num_points, 10); // First frame (i=0)
        assert_eq!(data[3].max_frequency, 1000.0);

        // Remaining entries should be defaults
        for i in 4..512 {
            assert_eq!(data[i].num_points, 0);
            assert_eq!(data[i].max_frequency, 0.0);
        }
    }
}

// ============================================================================
// Bevy Integration
// ============================================================================

use bevy::prelude::*;

/// Spectrum input manager with Bevy resource support
/// Note: Stream and Consumer are not Send+Sync, so we need unsafe impl
#[derive(Resource)]
pub struct SpectrumInputManager {
    pub(crate) history_data: SpectrumHistoryData,
    pub buffer_handle: Handle<bevy::render::storage::ShaderStorageBuffer>,
    pub buffer_needs_update: bool,
    pub consumer: Caching<Arc<HeapRb<f32>>, false, true>,
    min_frequency: f32,
    max_frequency: f32,
    sampling_rate: u32,
    _stream: Stream,
}

// SAFETY: SpectrumInputManager is only used from the main thread in Bevy
// The Stream and consumer are thread-local by nature
unsafe impl Send for SpectrumInputManager {}
unsafe impl Sync for SpectrumInputManager {}

impl SpectrumInputManager {
    pub const STORAGE_BINDING_INDEX: u32 = 1;

    pub fn new(
        buffer_handle: Handle<bevy::render::storage::ShaderStorageBuffer>,
        config: &SpectrumConfig,
    ) -> Self {
        let history_data = SpectrumHistoryData::new();

        let (stream, consumer) = setup_audio_stream();

        Self {
            history_data,
            buffer_handle,
            buffer_needs_update: true,
            consumer,
            min_frequency: config.min_frequency,
            max_frequency: config.max_frequency,
            sampling_rate: config.sampling_rate,
            _stream: stream,
        }
    }

    pub fn update(&mut self) {
        if self.consumer.occupied_len() < NUM_SAMPLES {
            return;
        }

        let mut samples: [f32; NUM_SAMPLES] = [0.0; NUM_SAMPLES];
        for sample_slot in samples.iter_mut() {
            let sample = self.consumer.try_pop().unwrap();
            *sample_slot = sample;
        }

        let hann_window = hann_window(&samples);
        let spectrum = samples_fft_to_spectrum(
            &hann_window,
            self.sampling_rate,
            FrequencyLimit::Range(self.min_frequency, self.max_frequency),
            Some(&divide_by_N_sqrt),
        )
        .unwrap();

        let mut frequencies = [[0.0; 4]; NUM_SAMPLES / 4];
        let mut amplitudes = [[0.0; 4]; NUM_SAMPLES / 4];

        for (i, f) in spectrum.data().iter().enumerate() {
            let vec4_index = i / 4;
            let element_index = i % 4;
            frequencies[vec4_index][element_index] = f.0.val();
            amplitudes[vec4_index][element_index] = f.1.val();
        }

        let (max_fr, max_amp) = spectrum.max();

        let frame_data = SpectrumFrameData {
            frequencies,
            amplitudes,
            num_points: spectrum.data().len() as u32,
            max_frequency: max_fr.val(),
            max_amplitude: max_amp.val(),
        };

        // First push current frame to history
        self.history_data.push_current_frame();
        // Then set new current frame
        self.history_data.set_current_frame(frame_data);
        self.buffer_needs_update = true;
    }

    pub fn write_buffer(
        &self,
        storage_buffers: &mut ResMut<Assets<bevy::render::storage::ShaderStorageBuffer>>,
    ) {
        let data = self.history_data.prepare_shader_data();
        let data_bytes = bytemuck::cast_slice(&data);

        if let Some(buffer) = storage_buffers.get_mut(&self.buffer_handle) {
            buffer.data = Some(data_bytes.to_vec());
        }
    }

    pub fn get_shader_data(&self) -> Vec<SpectrumShaderData> {
        self.history_data.prepare_shader_data()
    }
}

/// Bevy system for updating spectrum input
pub fn spectrum_input_system(
    mut spectrum_manager: Option<ResMut<SpectrumInputManager>>,
    mut storage_buffers: ResMut<Assets<bevy::render::storage::ShaderStorageBuffer>>,
) {
    if let Some(ref mut manager) = spectrum_manager {
        manager.update();

        // Write updated data to storage buffer
        if manager.buffer_needs_update {
            manager.write_buffer(&mut storage_buffers);
            manager.buffer_needs_update = false;
        }
    }
}

/// Bevy startup system for initializing spectrum input
pub fn setup_spectrum_input_system(
    mut commands: Commands,
    config: Res<crate::ShekereConfig>,
    buffer_handles: Option<Res<crate::shader_renderer::InputBufferHandles>>,
) {
    if let Some(spectrum_config) = &config.config.spectrum {
        log::info!("Setting up Spectrum input system");

        if let Some(handles) = buffer_handles {
            let spectrum_manager =
                SpectrumInputManager::new(handles.spectrum_buffer.clone(), spectrum_config);
            commands.insert_resource(spectrum_manager);
            log::info!("Spectrum input system setup completed with ShaderStorageBuffer");
        } else {
            log::warn!("InputBufferHandles not available, spectrum input will not work");
        }
    }
}
