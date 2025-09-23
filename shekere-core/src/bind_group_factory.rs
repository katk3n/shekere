use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, Device,
};

pub struct BindGroupFactory<'a> {
    pub entries: Vec<BindGroupEntry<'a>>,
    pub layout_entries: Vec<BindGroupLayoutEntry>,
}

impl<'a> BindGroupFactory<'a> {
    pub fn new() -> Self {
        Self {
            entries: vec![],
            layout_entries: vec![],
        }
    }

    pub fn add_entry(&mut self, binding_index: u32, buffer: &'a Buffer) {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: binding_index,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self.entries.push(wgpu::BindGroupEntry {
            binding: binding_index,
            resource: buffer.as_entire_binding(),
        })
    }

    pub fn add_storage_entry(&mut self, binding_index: u32, buffer: &'a Buffer) {
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: binding_index,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });
        self.entries.push(wgpu::BindGroupEntry {
            binding: binding_index,
            resource: buffer.as_entire_binding(),
        })
    }

    /// Creates a texture bind group layout using the standard multi-pass pattern
    pub fn create_multipass_texture_layout(device: &Device, label: &str) -> BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                // Binding 0: Texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // Binding 1: Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some(label),
        })
    }

    pub fn create(
        &self,
        device: &Device,
        label_prefix: &str,
    ) -> (Option<BindGroupLayout>, Option<BindGroup>) {
        if self.entries.is_empty() {
            return (None, None);
        }

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &self.layout_entries,
            label: Some(&format!("{}_bind_group_layout", label_prefix)),
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &self.entries,
            label: Some(&format!("{}_bind_group", label_prefix)),
        });
        (Some(bind_group_layout), Some(bind_group))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock texture and sampler for testing
    struct MockTexture {
        texture: wgpu::Texture,
        sampler: wgpu::Sampler,
    }

    impl MockTexture {
        fn new(device: &wgpu::Device) -> Self {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width: 256,
                    height: 256,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                label: Some("Mock Texture"),
                view_formats: &[],
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            Self { texture, sampler }
        }
    }

    fn create_test_device() -> (wgpu::Device, wgpu::Queue) {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::PRIMARY,
                ..Default::default()
            });

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();

            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        label: None,
                        memory_hints: Default::default(),
                    },
                    None,
                )
                .await
                .unwrap()
        })
    }

    #[test]
    fn test_bind_group_factory_create_multipass_texture_layout() {
        // Test creating the standard multipass texture layout
        let (device, _queue) = create_test_device();

        let layout = BindGroupFactory::create_multipass_texture_layout(&device, "test_layout");

        // Should successfully create a layout (the fact that this doesn't panic is sufficient)
        // Testing internal structure would require exposing private details
        drop(layout); // Just ensure it's a valid layout object
    }
}
