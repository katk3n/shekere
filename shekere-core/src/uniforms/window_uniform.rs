use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct WindowUniformData {
    resolution: [f32; 2],
}

pub struct WindowUniform {
    pub data: WindowUniformData,
    pub buffer: wgpu::Buffer,
}

impl WindowUniform {
    pub const BINDING_INDEX: u32 = 0;

    /// Create a WindowUniform with explicit width and height (without requiring a Window)
    pub fn new_with_size(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let data = WindowUniformData {
            resolution: [width as f32, height as f32],
        };
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Window Buffer"),
            contents: bytemuck::cast_slice(&[data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self { data, buffer }
    }

    /// Update window size without requiring a Window reference
    pub fn update_size(&mut self, width: u32, height: u32) {
        self.data.resolution = [width as f32, height as f32];
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.data]));
    }
}

impl WindowUniformData {
    #[cfg(test)]
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            resolution: [width, height],
        }
    }

    pub fn width(&self) -> f32 {
        self.resolution[0]
    }

    pub fn height(&self) -> f32 {
        self.resolution[1]
    }

    pub fn aspect_ratio(&self) -> f32 {
        if self.resolution[1] != 0.0 {
            self.resolution[0] / self.resolution[1]
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_uniform_data_new() {
        let data = WindowUniformData::new(800.0, 600.0);
        assert_eq!(data.resolution, [800.0, 600.0]);
    }

    #[test]
    fn test_window_uniform_data_getters() {
        let data = WindowUniformData::new(1920.0, 1080.0);
        assert_eq!(data.width(), 1920.0);
        assert_eq!(data.height(), 1080.0);
    }

    #[test]
    fn test_window_uniform_data_aspect_ratio() {
        let data = WindowUniformData::new(800.0, 600.0);
        assert!((data.aspect_ratio() - 1.333333).abs() < 0.001);

        let square = WindowUniformData::new(500.0, 500.0);
        assert_eq!(square.aspect_ratio(), 1.0);

        let zero_height = WindowUniformData::new(800.0, 0.0);
        assert_eq!(zero_height.aspect_ratio(), 1.0);
    }

    #[test]
    fn test_window_uniform_data_bytemuck() {
        let data = WindowUniformData::new(1024.0, 768.0);
        let data_array = [data];
        let bytes: &[u8] = bytemuck::cast_slice(&data_array);
        assert_eq!(bytes.len(), 8);

        let reconstructed: WindowUniformData =
            bytemuck::cast_slice::<u8, WindowUniformData>(bytes)[0];
        assert_eq!(reconstructed.resolution, [1024.0, 768.0]);
    }

    #[test]
    fn test_window_uniform_data_equality() {
        let data1 = WindowUniformData::new(800.0, 600.0);
        let data2 = WindowUniformData::new(800.0, 600.0);
        let data3 = WindowUniformData::new(1024.0, 768.0);

        assert_eq!(data1, data2);
        assert_ne!(data1, data3);
    }
}
