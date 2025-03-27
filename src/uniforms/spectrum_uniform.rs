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
struct SpectrumDataPoint {
    frequency: f32,
    amplitude: f32,
    _padding: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpectrumUniformData {
    data_points: [SpectrumDataPoint; NUM_SAMPLES],
    num_frequencies: u32,
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
            data_points: [SpectrumDataPoint {
                frequency: 0.0,
                amplitude: 0.0,
                _padding: [0; 2],
            }; NUM_SAMPLES],
            num_frequencies: 0,
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

        let mut data_points = [SpectrumDataPoint {
            frequency: 0.0,
            amplitude: 0.0,
            _padding: [0; 2],
        }; NUM_SAMPLES];

        for (i, f) in spectrum.data().iter().enumerate() {
            data_points[i].frequency = f.0.val();
            data_points[i].amplitude = f.1.val();
            println!(
                "fr: {}, amp: {}",
                data_points[i].frequency, data_points[i].amplitude
            );
        }

        let (max_fr, max_amp) = spectrum.max();

        self.data.data_points = data_points;
        self.data.num_frequencies = spectrum.data().len() as u32;
        self.data.max_frequency = max_fr.val();
        self.data.max_amplitude = max_amp.val();

        println!(
            "max_fr: {}, max_amp: {}",
            self.data.max_frequency, self.data.max_amplitude
        );
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
