use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
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

impl TimeUniformData {
    #[cfg(test)]
    pub fn new(duration: f32) -> Self {
        Self { duration }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_uniform_data_new() {
        let data = TimeUniformData::new(5.0);
        assert_eq!(data.duration, 5.0);
    }

    #[test]
    fn test_time_uniform_data_default() {
        let data = TimeUniformData { duration: 0.0 };
        assert_eq!(data.duration, 0.0);
    }

    #[test]
    fn test_time_uniform_data_bytemuck() {
        let data = TimeUniformData::new(1.5);
        let data_array = [data];
        let bytes: &[u8] = bytemuck::cast_slice(&data_array);
        assert_eq!(bytes.len(), 4);

        let reconstructed: TimeUniformData = bytemuck::cast_slice::<u8, TimeUniformData>(bytes)[0];
        assert_eq!(reconstructed.duration, 1.5);
    }

    #[test]
    fn test_time_uniform_data_equality() {
        let data1 = TimeUniformData::new(2.0);
        let data2 = TimeUniformData::new(2.0);
        let data3 = TimeUniformData::new(3.0);

        assert_eq!(data1, data2);
        assert_ne!(data1, data3);
    }
}
