use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
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

impl MouseUniformData {
    #[cfg(test)]
    pub fn new(x: f32, y: f32) -> Self {
        Self { position: [x, y] }
    }

    #[cfg(test)]
    pub fn x(&self) -> f32 {
        self.position[0]
    }

    #[cfg(test)]
    pub fn y(&self) -> f32 {
        self.position[1]
    }

    #[cfg(test)]
    pub fn normalized(&self, width: f32, height: f32) -> [f32; 2] {
        [
            if width > 0.0 {
                self.position[0] / width
            } else {
                0.0
            },
            if height > 0.0 {
                self.position[1] / height
            } else {
                0.0
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_uniform_data_new() {
        let data = MouseUniformData::new(100.0, 200.0);
        assert_eq!(data.position, [100.0, 200.0]);
    }

    #[test]
    fn test_mouse_uniform_data_getters() {
        let data = MouseUniformData::new(150.0, 300.0);
        assert_eq!(data.x(), 150.0);
        assert_eq!(data.y(), 300.0);
    }

    #[test]
    fn test_mouse_uniform_data_normalized() {
        let data = MouseUniformData::new(400.0, 300.0);
        let normalized = data.normalized(800.0, 600.0);
        assert_eq!(normalized, [0.5, 0.5]);

        let normalized_zero_dims = data.normalized(0.0, 0.0);
        assert_eq!(normalized_zero_dims, [0.0, 0.0]);
    }

    #[test]
    fn test_mouse_uniform_data_bytemuck() {
        let data = MouseUniformData::new(123.45, 678.90);
        let data_array = [data];
        let bytes: &[u8] = bytemuck::cast_slice(&data_array);
        assert_eq!(bytes.len(), 8);

        let reconstructed: MouseUniformData =
            bytemuck::cast_slice::<u8, MouseUniformData>(bytes)[0];
        assert_eq!(reconstructed.position, [123.45, 678.90]);
    }

    #[test]
    fn test_mouse_uniform_data_equality() {
        let data1 = MouseUniformData::new(100.0, 200.0);
        let data2 = MouseUniformData::new(100.0, 200.0);
        let data3 = MouseUniformData::new(150.0, 250.0);

        assert_eq!(data1, data2);
        assert_ne!(data1, data3);
    }
}
