use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, Device, Sampler, TextureView,
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

    pub fn add_texture_entry(
        &mut self,
        texture_binding: u32,
        sampler_binding: u32,
        texture_view: &'a TextureView,
        sampler: &'a Sampler,
    ) {
        // Add texture binding
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: texture_binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            count: None,
        });
        self.entries.push(wgpu::BindGroupEntry {
            binding: texture_binding,
            resource: wgpu::BindingResource::TextureView(texture_view),
        });

        // Add sampler binding
        self.layout_entries.push(wgpu::BindGroupLayoutEntry {
            binding: sampler_binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        self.entries.push(wgpu::BindGroupEntry {
            binding: sampler_binding,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
    }

    /// Helper method for the common multi-pass texture pattern (texture at binding 0, sampler at binding 1)
    pub fn add_multipass_texture(&mut self, texture_view: &'a TextureView, sampler: &'a Sampler) {
        self.add_texture_entry(0, 1, texture_view, sampler);
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
    fn test_bind_group_factory_texture_entry() {
        // TDD Green phase: Now we can test the add_texture_entry method
        let (device, _queue) = create_test_device();
        let mock_texture = MockTexture::new(&device);

        let mut factory = BindGroupFactory::new();

        // Add texture entry with binding 0 for texture and binding 1 for sampler
        let texture_view = mock_texture.texture.create_view(&Default::default());
        factory.add_texture_entry(0, 1, &texture_view, &mock_texture.sampler);

        // Should have 2 entries (texture + sampler)
        assert_eq!(factory.entries.len(), 2);
        assert_eq!(factory.layout_entries.len(), 2);

        // Check texture binding
        assert_eq!(factory.layout_entries[0].binding, 0);
        assert_eq!(factory.entries[0].binding, 0);

        // Check sampler binding
        assert_eq!(factory.layout_entries[1].binding, 1);
        assert_eq!(factory.entries[1].binding, 1);
    }

    #[test]
    fn test_bind_group_factory_mixed_entries() {
        // Test mixing buffer and texture entries
        let (device, _queue) = create_test_device();
        let mock_texture = MockTexture::new(&device);

        // Create a simple buffer
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Test Buffer"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut factory = BindGroupFactory::new();

        // Add buffer entry
        factory.add_entry(0, &buffer);

        // Add texture entry
        let texture_view = mock_texture.texture.create_view(&Default::default());
        factory.add_texture_entry(1, 2, &texture_view, &mock_texture.sampler);

        // Should have 3 entries (buffer + texture + sampler)
        assert_eq!(factory.entries.len(), 3);
        assert_eq!(factory.layout_entries.len(), 3);

        // Create bind group to ensure it works
        let (layout, bind_group) = factory.create(&device, "test");
        assert!(layout.is_some());
        assert!(bind_group.is_some());
    }

    #[test]
    fn test_bind_group_factory_add_multipass_texture() {
        // Test the helper method for multipass texture pattern
        let (device, _queue) = create_test_device();
        let mock_texture = MockTexture::new(&device);

        let mut factory = BindGroupFactory::new();

        // Add multipass texture using helper
        let texture_view = mock_texture.texture.create_view(&Default::default());
        factory.add_multipass_texture(&texture_view, &mock_texture.sampler);

        // Should have 2 entries (texture at binding 0, sampler at binding 1)
        assert_eq!(factory.entries.len(), 2);
        assert_eq!(factory.layout_entries.len(), 2);

        // Check texture binding is at 0
        assert_eq!(factory.layout_entries[0].binding, 0);
        assert_eq!(factory.entries[0].binding, 0);

        // Check sampler binding is at 1
        assert_eq!(factory.layout_entries[1].binding, 1);
        assert_eq!(factory.entries[1].binding, 1);
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
