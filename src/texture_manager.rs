use std::collections::HashMap;
use wgpu::{
    Device, Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureFormat, TextureUsages,
    TextureView,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureType {
    Intermediate,
    PingPong,
    Persistent,
}

pub struct TextureManager {
    width: u32,
    height: u32,
    format: TextureFormat,
    intermediate_textures: HashMap<usize, (Texture, TextureView)>,
    ping_pong_textures: HashMap<usize, [(Texture, TextureView); 2]>,
    pub persistent_textures: HashMap<usize, [(Texture, TextureView); 2]>,
    persistent_initialized: HashMap<usize, bool>,
    pub sampler: Sampler,
    pub current_frame: u64,
}

impl TextureManager {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        Self::new_with_format(device, width, height, TextureFormat::Rgba8Unorm)
    }

    pub fn new_with_format(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Self {
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            width,
            height,
            format,
            intermediate_textures: HashMap::new(),
            ping_pong_textures: HashMap::new(),
            persistent_textures: HashMap::new(),
            persistent_initialized: HashMap::new(),
            sampler,
            current_frame: 0,
        }
    }

    pub fn update_size(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.clear_all_textures();
        }
    }

    pub fn clear_all_textures(&mut self) {
        self.intermediate_textures.clear();
        self.ping_pong_textures.clear();
        self.persistent_textures.clear();
        self.persistent_initialized.clear();
        // Reset frame counter to ensure consistent double-buffering
        self.current_frame = 0;
    }

    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }

    fn create_texture(&self, device: &Device, label: &str) -> (Texture, TextureView) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    pub fn get_or_create_intermediate_texture(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        if !self.intermediate_textures.contains_key(&pass_index) {
            let (texture, view) =
                self.create_texture(device, &format!("Intermediate Texture {}", pass_index));
            self.intermediate_textures
                .insert(pass_index, (texture, view));
        }
        let (_, view) = self.intermediate_textures.get(&pass_index).unwrap();
        (view, &self.sampler)
    }

    pub fn get_or_create_ping_pong_texture(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        if !self.ping_pong_textures.contains_key(&pass_index) {
            let (texture_a, view_a) =
                self.create_texture(device, &format!("Ping Pong Texture A {}", pass_index));
            let (texture_b, view_b) =
                self.create_texture(device, &format!("Ping Pong Texture B {}", pass_index));
            self.ping_pong_textures
                .insert(pass_index, [(texture_a, view_a), (texture_b, view_b)]);
        }

        let textures = self.ping_pong_textures.get(&pass_index).unwrap();
        let current_index = (self.current_frame % 2) as usize;
        let (_, view) = &textures[current_index];
        (view, &self.sampler)
    }

    pub fn get_or_create_persistent_texture(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        if !self.persistent_textures.contains_key(&pass_index) {
            let (texture_a, view_a) =
                self.create_texture(device, &format!("Persistent Texture A {}", pass_index));
            let (texture_b, view_b) =
                self.create_texture(device, &format!("Persistent Texture B {}", pass_index));
            self.persistent_textures
                .insert(pass_index, [(texture_a, view_a), (texture_b, view_b)]);
            self.persistent_initialized.insert(pass_index, false);
        }

        // Return the read texture (previous frame)
        // Use proper double-buffering: read from previous frame, write to current frame
        let textures = self.persistent_textures.get(&pass_index).unwrap();
        let read_index = ((self.current_frame + 1) % 2) as usize; // Read from previous frame
        let (_, view) = &textures[read_index];
        (view, &self.sampler)
    }

    pub fn is_persistent_texture_initialized(&self, pass_index: usize) -> bool {
        self.persistent_initialized
            .get(&pass_index)
            .copied()
            .unwrap_or(false)
    }

    pub fn mark_persistent_texture_initialized(&mut self, pass_index: usize) {
        self.persistent_initialized.insert(pass_index, true);
    }

    pub fn get_ping_pong_render_target(&self, pass_index: usize) -> Option<&TextureView> {
        self.ping_pong_textures.get(&pass_index).map(|textures| {
            let next_index = ((self.current_frame + 1) % 2) as usize;
            &textures[next_index].1
        })
    }

    pub fn get_intermediate_render_target(&self, pass_index: usize) -> Option<&TextureView> {
        self.intermediate_textures
            .get(&pass_index)
            .map(|(_, view)| view)
    }

    pub fn get_persistent_render_target(&self, pass_index: usize) -> Option<&TextureView> {
        self.persistent_textures.get(&pass_index).map(|textures| {
            // Return the write texture (current frame)
            // For double-buffering: read from previous frame, write to current frame
            let write_index = (self.current_frame % 2) as usize; // Write to current frame
            log::info!("Persistent texture output: frame={}, write_index={}", 
                      self.current_frame, write_index);
            &textures[write_index].1
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_device() -> Device {
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
                .0
        })
    }

    #[test]
    fn test_texture_manager_creation() {
        let device = create_test_device();
        let manager = TextureManager::new(&device, 800, 600);

        assert_eq!(manager.width, 800);
        assert_eq!(manager.height, 600);
        assert_eq!(manager.format, TextureFormat::Rgba8Unorm);
        assert_eq!(manager.current_frame, 0);
    }

    #[test]
    fn test_intermediate_texture_creation() {
        let device = create_test_device();
        let mut manager = TextureManager::new(&device, 800, 600);

        let (view, sampler) = manager.get_or_create_intermediate_texture(&device, 0);
        // TextureView and Sampler don't have is_empty() method, so we check they exist
        assert!(view as *const _ != std::ptr::null());
        assert!(sampler as *const _ != std::ptr::null());

        // Store pointer values to compare later
        let view_ptr = view as *const _;
        let sampler_ptr = sampler as *const _;

        // Should return the same texture on subsequent calls
        let (view2, sampler2) = manager.get_or_create_intermediate_texture(&device, 0);
        assert_eq!(view_ptr, view2 as *const _);
        assert_eq!(sampler_ptr, sampler2 as *const _);
    }

    #[test]
    fn test_ping_pong_texture_creation() {
        let device = create_test_device();
        let mut manager = TextureManager::new(&device, 800, 600);

        let (view1, sampler1) = manager.get_or_create_ping_pong_texture(&device, 0);
        assert!(view1 as *const _ != std::ptr::null());
        assert!(sampler1 as *const _ != std::ptr::null());

        // Store pointer values to compare later
        let view1_ptr = view1 as *const _;
        let sampler1_ptr = sampler1 as *const _;

        // Advance frame and check we get different texture
        manager.advance_frame();
        let (view2, sampler2) = manager.get_or_create_ping_pong_texture(&device, 0);
        assert_ne!(view1_ptr, view2 as *const _);
        assert_eq!(sampler1_ptr, sampler2 as *const _);
    }

    #[test]
    fn test_persistent_texture_creation() {
        let device = create_test_device();
        let mut manager = TextureManager::new(&device, 800, 600);

        let (view, sampler) = manager.get_or_create_persistent_texture(&device, 0);
        assert!(view as *const _ != std::ptr::null());
        assert!(sampler as *const _ != std::ptr::null());

        // Store pointer values to compare later
        let view_ptr = view as *const _;
        let sampler_ptr = sampler as *const _;

        // Should return the same texture in the same frame
        let (view2, sampler2) = manager.get_or_create_persistent_texture(&device, 0);
        assert_eq!(view_ptr, view2 as *const _);
        assert_eq!(sampler_ptr, sampler2 as *const _);

        // After frame advance, should return different read texture
        manager.advance_frame();
        let (view3, sampler3) = manager.get_or_create_persistent_texture(&device, 0);
        assert_ne!(view_ptr, view3 as *const _);
        assert_eq!(sampler_ptr, sampler3 as *const _);
    }

    #[test]
    fn test_texture_manager_resize() {
        let device = create_test_device();
        let mut manager = TextureManager::new(&device, 800, 600);

        // Create a texture first
        {
            let (view1, _) = manager.get_or_create_intermediate_texture(&device, 0);
            assert!(view1 as *const _ != std::ptr::null());
        }

        // Verify texture exists
        assert_eq!(manager.intermediate_textures.len(), 1);

        // Resize should clear all textures
        manager.update_size(1024, 768);
        assert_eq!(manager.width, 1024);
        assert_eq!(manager.height, 768);

        // Verify textures were cleared
        assert_eq!(manager.intermediate_textures.len(), 0);

        // Should create a new texture with new size
        {
            let (view2, _) = manager.get_or_create_intermediate_texture(&device, 0);
            assert!(view2 as *const _ != std::ptr::null());
        }
        assert_eq!(manager.intermediate_textures.len(), 1);
    }

    #[test]
    fn test_render_target_access() {
        let device = create_test_device();
        let mut manager = TextureManager::new(&device, 800, 600);

        // Create ping-pong textures
        let (_, _) = manager.get_or_create_ping_pong_texture(&device, 0);

        // Get render target
        let render_target = manager.get_ping_pong_render_target(0);
        assert!(render_target.is_some());
        let render_target_ptr = render_target.unwrap() as *const _;

        // Advance frame and check render target changes
        manager.advance_frame();
        let render_target2 = manager.get_ping_pong_render_target(0);
        assert!(render_target2.is_some());
        assert_ne!(render_target_ptr, render_target2.unwrap() as *const _);
    }
}
