use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalPosition, window::Window};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WindowUniform {
    resolution: [f32; 2],
}

impl WindowUniform {
    pub const BINDING_INDEX: u32 = 0;
    pub fn new(window: &Window) -> Self {
        Self {
            resolution: [
                window.inner_size().width as f32,
                window.inner_size().height as f32,
            ],
        }
    }

    pub fn update(&mut self, window: &Window) {
        self.resolution = [
            window.inner_size().width as f32,
            window.inner_size().height as f32,
        ];
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Window Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniform {
    duration: f32,
}

impl TimeUniform {
    pub const BINDING_INDEX: u32 = 1;
    pub fn new() -> Self {
        Self { duration: 0.0 }
    }

    pub fn update(&mut self, duration: f32) {
        self.duration = duration;
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Time Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MouseUniform {
    position: [f32; 2],
}

impl MouseUniform {
    pub const BINDING_INDEX: u32 = 0;
    pub fn new() -> Self {
        Self {
            position: [0.0, 0.0],
        }
    }

    pub fn update(&mut self, position: &PhysicalPosition<f64>) {
        self.position = [position.x as f32, position.y as f32];
    }

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Mouse Buffer"),
            contents: bytemuck::cast_slice(&[*self]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }
}
