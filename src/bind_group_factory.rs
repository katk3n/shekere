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

    pub fn create(
        &self,
        device: &Device,
        label_prefix: &str,
    ) -> (Option<BindGroupLayout>, Option<BindGroup>) {
        if self.entries.len() == 0 {
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
