use std::sync::Arc;

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream,
};
use ringbuf::{
    traits::{Producer, Split},
    wrap::caching::Caching,
    HeapRb,
};

pub const NUM_SAMPLES: usize = 2048;

pub fn setup_audio_stream() -> (Stream, Caching<Arc<HeapRb<f32>>, false, true>) {
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
                    match prod.try_push(*d) {
                        Ok(()) => {}
                        Err(_) => {}
                    }
                }
            },
            move |_err| {},
            None,
        )
        .unwrap();

    stream.play().unwrap();
    return (stream, cons);
}
