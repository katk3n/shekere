use std::sync::Arc;

use cpal::Stream;
use ringbuf::{
    traits::{Consumer, Observer},
    wrap::caching::Caching,
    HeapRb,
};
use spectrum_analyzer::scaling::*;
use spectrum_analyzer::windows::hann_window;
use spectrum_analyzer::{samples_fft_to_spectrum, FrequencyLimit};
use wgpu::util::DeviceExt;

use crate::{
    audio_stream::{self, NUM_SAMPLES},
    config::SpectrumConfig,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpectrumUniformData {
    // Packed into vec4s for WebGPU alignment (NUM_SAMPLES/4 = 512)
    frequencies: [[f32; 4]; NUM_SAMPLES / 4],
    amplitudes: [[f32; 4]; NUM_SAMPLES / 4],
    num_points: u32,
    max_frequency: f32,
    max_amplitude: f32,
    _padding: u32,
}

pub struct SpectrumUniform {
    pub data: SpectrumUniformData,
    pub buffer: wgpu::Buffer,
    pub consumer: Caching<Arc<HeapRb<f32>>, false, true>,
    min_frequency: f32,
    max_frequency: f32,
    sampling_rate: u32,
    _stream: Stream,
}

impl SpectrumUniform {
    pub const BINDING_INDEX: u32 = 1;

    pub fn new(device: &wgpu::Device, config: &SpectrumConfig) -> Self {
        let data = SpectrumUniformData {
            frequencies: [[0.0; 4]; NUM_SAMPLES / 4],
            amplitudes: [[0.0; 4]; NUM_SAMPLES / 4],
            num_points: 0,
            max_frequency: 0.0,
            max_amplitude: 0.0,
            _padding: 0,
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Audio Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let (stream, consumer) = audio_stream::setup_audio_stream();

        Self {
            data,
            buffer,
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
        for i in 0..NUM_SAMPLES {
            let sample = self.consumer.try_pop().unwrap();
            samples[i] = sample;
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

        self.data.frequencies = frequencies;
        self.data.amplitudes = amplitudes;
        self.data.num_points = spectrum.data().len() as u32;
        self.data.max_frequency = max_fr.val();
        self.data.max_amplitude = max_amp.val();
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
