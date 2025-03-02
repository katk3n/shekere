use std::collections::HashMap;

use crate::{config::OscConfig, osc};
use async_std::channel::Receiver;
use rosc::{OscMessage, OscPacket, OscType};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct OscTruck {
    sound: i32,
    ttl: f32,
    note: f32,
    gain: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OscUniformData {
    trucks: [OscTruck; 16],
}

pub struct OscUniform<'a> {
    pub data: OscUniformData,
    pub buffer: wgpu::Buffer,
    pub sound_map: HashMap<&'a str, i32>,
    pub receiver: Receiver<OscPacket>,
}

impl<'a> OscUniform<'a> {
    pub const BINDING_INDEX: u32 = 0;

    pub async fn new(device: &wgpu::Device, config: &'a OscConfig) -> Self {
        let data = OscUniformData {
            trucks: [OscTruck {
                sound: 0,
                ttl: 0.0,
                note: 0.0,
                gain: 0.0,
            }; 16],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Osc Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let mut sound_map = HashMap::new();
        for s in &config.sound {
            sound_map.insert(s.name.as_str(), s.id);
        }
        let receiver = osc::osc_start(config.port).await;
        Self {
            data,
            buffer,
            sound_map,
            receiver,
        }
    }

    fn handle_message(&mut self, msg: &OscMessage) {
        //println!("OSC msg: {} {:?}", msg.addr, msg.args);
        let mut id: usize = 0;
        let mut ttl = 0.0;
        let mut note = 0.0;
        let mut gain = 0.0;
        let mut sound = 0;
        for (i, v) in msg.args.iter().enumerate() {
            match v {
                OscType::String(val) => match val.as_str() {
                    "orbit" => {
                        let orbit = &msg.args[i + 1];
                        if let OscType::Int(orbit) = orbit {
                            id = *orbit as usize;
                        }
                    }
                    "delta" => {
                        let delta = &msg.args[i + 1];
                        if let OscType::Float(delta) = delta {
                            ttl = *delta;
                        }
                    }
                    "note" | "n" => {
                        let n = &msg.args[i + 1];
                        if let OscType::Float(n) = n {
                            note = *n;
                        }
                    }
                    "gain" => {
                        let g = &msg.args[i + 1];
                        if let OscType::Float(g) = g {
                            gain = *g;
                        }
                    }
                    "sound" | "s" => {
                        let s = &msg.args[i + 1];
                        if let OscType::String(s) = s {
                            if let Some(sound_id) = self.sound_map.get(s.as_str()) {
                                sound = *sound_id;
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        self.data.trucks[id].sound = sound;
        self.data.trucks[id].ttl = ttl;
        self.data.trucks[id].note = note;
        self.data.trucks[id].gain = gain;
    }

    pub fn update(&mut self, time_elapsed: f32) {
        match self.receiver.try_recv() {
            Ok(packet) => {
                if let OscPacket::Bundle(bundle) = packet {
                    let content = &bundle.content[0];
                    if let OscPacket::Message(msg) = content {
                        self.handle_message(msg);
                    }
                }
            }
            Err(_) => {
                self.elapse(time_elapsed);
            }
        }
    }

    pub fn elapse(&mut self, time_delta: f32) {
        let mut trucks = [OscTruck {
            ttl: 0.0,
            sound: 0,
            note: 0.0,
            gain: 0.0,
        }; 16];
        for (i, tr) in self.data.trucks.iter().enumerate() {
            let t = tr.ttl - time_delta;
            if t > 0.0 {
                trucks[i].ttl = t;
                trucks[i].sound = tr.sound;
                trucks[i].note = tr.note;
                trucks[i].gain = tr.gain;
            }
        }
        self.data.trucks = trucks;
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
