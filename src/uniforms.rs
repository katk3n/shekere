use winit::{dpi::PhysicalPosition, window::Window};

pub const WINDOW_BINDING_INDEX: u32 = 0;
pub const TIME_BINDING_INDEX: u32 = 1;
pub const MOUSE_BINDING_INDEX: u32 = 0;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WindowUniform {
    resolution: [f32; 2],
}

impl WindowUniform {
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
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniform {
    duration: f32,
}

impl TimeUniform {
    pub fn new() -> Self {
        Self { duration: 0.0 }
    }

    pub fn update(&mut self, duration: f32) {
        self.duration = duration;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MouseUniform {
    position: [f32; 2],
}

impl MouseUniform {
    pub fn new() -> Self {
        Self {
            position: [0.0, 0.0],
        }
    }

    pub fn update(&mut self, position: &PhysicalPosition<f64>) {
        self.position = [position.x as f32, position.y as f32];
    }
}
