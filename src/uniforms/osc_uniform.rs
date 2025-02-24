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

pub struct OscUniform {
    pub data: OscUniformData,
    pub buffer: wgpu::Buffer,
}

impl OscUniform {
    pub const BINDING_INDEX: u32 = 0;

    pub fn new(device: &wgpu::Device) -> Self {
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
        Self { data, buffer }
    }

    fn handle_message(&mut self, msg: &OscMessage) {
        println!("OSC msg: {} {:?}", msg.addr, msg.args);
        let mut id: usize = 0;
        let mut ttl = 0.0;
        let mut note = 0.0;
        let mut gain = 0.0;
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
                    _ => {}
                },
                _ => {}
            }
        }
        self.data.trucks[id].ttl = ttl;
        self.data.trucks[id].note = note;
        self.data.trucks[id].gain = gain;
    }

    pub fn update(&mut self, packet: OscPacket) {
        match packet {
            OscPacket::Message(msg) => {
                println!("OSC msg: {} {:?}", msg.addr, msg.args);
            }
            OscPacket::Bundle(bundle) => {
                let content = &bundle.content[0];
                if let OscPacket::Message(msg) = content {
                    self.handle_message(msg);
                }
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
