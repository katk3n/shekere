use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniformData {
    pub duration: f32,
}

pub struct TimeUniform {
    pub data: TimeUniformData,
    pub buffer: wgpu::Buffer,
}

impl TimeUniform {
    pub const BINDING_INDEX: u32 = 1;

    pub fn new(device: &wgpu::Device) -> Self {
        let data = TimeUniformData { duration: 0.0 };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Time Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { data, buffer }
    }

    pub fn update(&mut self, duration: f32) {
        self.data.duration = duration;
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
