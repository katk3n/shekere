use wgpu::util::DeviceExt;
use winit::window::Window;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WindowUniformData {
    resolution: [f32; 2],
}

pub struct WindowUniform {
    pub data: WindowUniformData,
    pub buffer: wgpu::Buffer,
}

impl WindowUniform {
    pub const BINDING_INDEX: u32 = 0;

    pub fn new(device: &wgpu::Device, window: &Window) -> Self {
        let data = WindowUniformData {
            resolution: [
                window.inner_size().width as f32,
                window.inner_size().height as f32,
            ],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Window Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { data, buffer }
    }

    pub fn update(&mut self, window: &Window) {
        self.data.resolution = [
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        ];
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}
