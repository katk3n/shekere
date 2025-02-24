use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MouseUniformData {
    position: [f32; 2],
}

pub struct MouseUniform {
    pub data: MouseUniformData,
    pub buffer: wgpu::Buffer,
}

impl MouseUniform {
    pub const BINDING_INDEX: u32 = 0;

    pub fn new(device: &wgpu::Device) -> Self {
        let data = MouseUniformData {
            position: [0.0, 0.0],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mouse Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { data, buffer }
    }

    pub fn update(&mut self, position: &PhysicalPosition<f64>) {
        self.data.position = [position.x as f32, position.y as f32];
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
