/// WASM-compatible TextureManager for browser-based multi-pass rendering
/// Handles intermediate, ping-pong, and persistent textures for complex shader effects

use crate::render_constants::frame_buffer;
use wasm_bindgen::prelude::*;
use wgpu::{Device, Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureFormat,
          TextureUsages, TextureView, TextureViewDescriptor, BindGroup, BindGroupDescriptor,
          BindGroupEntry, BindingResource};
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

/// Types of textures supported by the texture manager
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmTextureType {
    Intermediate,
    PingPong,
    Persistent,
}

/// WASM-compatible TextureManager for browser-based rendering
pub struct WasmTextureManager {
    width: u32,
    height: u32,
    format: TextureFormat,
    intermediate_textures: HashMap<usize, (Texture, TextureView)>,
    pub ping_pong_textures: HashMap<usize, [(Texture, TextureView); 2]>,
    ping_pong_initialized: HashMap<usize, bool>,
    pub persistent_textures: HashMap<usize, [(Texture, TextureView); 2]>,
    persistent_initialized: HashMap<usize, bool>,
    pub sampler: Sampler,
    pub current_frame: u64,

    // WASM-specific: Cache bind groups for multipass textures
    intermediate_bind_groups: HashMap<usize, BindGroup>,
    ping_pong_bind_groups: HashMap<usize, [BindGroup; 2]>,
    persistent_bind_groups: HashMap<usize, [BindGroup; 2]>,
}

impl WasmTextureManager {
    /// Create a new WASM TextureManager
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        Self::new_with_format(device, width, height, TextureFormat::Rgba8Unorm)
    }

    /// Create a new WASM TextureManager with specific format
    pub fn new_with_format(
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
    ) -> Self {
        console_log!("🖼️ Creating WASM TextureManager: {}x{}, format: {:?}", width, height, format);

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
            ping_pong_initialized: HashMap::new(),
            persistent_textures: HashMap::new(),
            persistent_initialized: HashMap::new(),
            sampler,
            current_frame: 0,
            intermediate_bind_groups: HashMap::new(),
            ping_pong_bind_groups: HashMap::new(),
            persistent_bind_groups: HashMap::new(),
        }
    }

    /// Update texture manager size (recreates all textures)
    pub fn update_size(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            console_log!("📏 Updating texture manager size: {}x{} -> {}x{}",
                       self.width, self.height, width, height);
            self.width = width;
            self.height = height;
            self.clear_all_textures();
        }
    }

    /// Clear all textures and bind groups
    pub fn clear_all_textures(&mut self) {
        console_log!("🧹 Clearing all textures and bind groups");
        self.intermediate_textures.clear();
        self.ping_pong_textures.clear();
        self.ping_pong_initialized.clear();
        self.persistent_textures.clear();
        self.persistent_initialized.clear();

        // Clear WASM-specific bind group caches
        self.intermediate_bind_groups.clear();
        self.ping_pong_bind_groups.clear();
        self.persistent_bind_groups.clear();

        // Reset frame counter to ensure consistent double-buffering
        self.current_frame = 0;
    }

    /// Advance to next frame (updates buffer indices)
    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Get current width
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get current height
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Create a texture with current settings
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

        let view = texture.create_view(&TextureViewDescriptor::default());
        (texture, view)
    }

    /// Create bind group for texture and sampler
    fn create_texture_bind_group(
        &self,
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        texture_view: &TextureView,
        label: &str,
    ) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Unified texture creation interface using strategy pattern
    pub fn get_or_create_texture(
        &mut self,
        device: &Device,
        texture_type: WasmTextureType,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        match texture_type {
            WasmTextureType::Intermediate => {
                self.create_intermediate_texture_strategy(device, pass_index)
            }
            WasmTextureType::PingPong => self.create_ping_pong_texture_strategy(device, pass_index),
            WasmTextureType::Persistent => self.create_persistent_texture_strategy(device, pass_index),
        }
    }

    /// Get or create intermediate texture bind group
    pub fn get_or_create_intermediate_bind_group(
        &mut self,
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        pass_index: usize,
    ) -> &BindGroup {
        // Ensure texture exists
        self.get_or_create_intermediate_texture(device, pass_index);

        if !self.intermediate_bind_groups.contains_key(&pass_index) {
            let (_, view) = self.intermediate_textures.get(&pass_index).unwrap();
            let bind_group = self.create_texture_bind_group(
                device,
                layout,
                view,
                &format!("Intermediate BindGroup {}", pass_index)
            );
            self.intermediate_bind_groups.insert(pass_index, bind_group);
        }

        self.intermediate_bind_groups.get(&pass_index).unwrap()
    }

    /// Get or create ping-pong texture bind group (returns read texture bind group)
    pub fn get_or_create_ping_pong_bind_group(
        &mut self,
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        pass_index: usize,
    ) -> &BindGroup {
        // Ensure texture exists
        self.get_or_create_ping_pong_texture(device, pass_index);

        if !self.ping_pong_bind_groups.contains_key(&pass_index) {
            let textures = self.ping_pong_textures.get(&pass_index).unwrap();

            // Create bind groups for both textures
            let bind_group_a = self.create_texture_bind_group(
                device,
                layout,
                &textures[0].1,
                &format!("PingPong BindGroup A {}", pass_index)
            );
            let bind_group_b = self.create_texture_bind_group(
                device,
                layout,
                &textures[1].1,
                &format!("PingPong BindGroup B {}", pass_index)
            );

            self.ping_pong_bind_groups.insert(pass_index, [bind_group_a, bind_group_b]);
        }

        // Return bind group for reading (previous frame)
        let bind_groups = self.ping_pong_bind_groups.get(&pass_index).unwrap();
        let read_index = frame_buffer::previous_buffer_index(self.current_frame);
        &bind_groups[read_index]
    }

    /// Get or create persistent texture bind group (returns read texture bind group)
    pub fn get_or_create_persistent_bind_group(
        &mut self,
        device: &Device,
        layout: &wgpu::BindGroupLayout,
        pass_index: usize,
    ) -> &BindGroup {
        // Ensure texture exists
        self.get_or_create_persistent_texture(device, pass_index);

        if !self.persistent_bind_groups.contains_key(&pass_index) {
            let textures = self.persistent_textures.get(&pass_index).unwrap();

            // Create bind groups for both textures
            let bind_group_a = self.create_texture_bind_group(
                device,
                layout,
                &textures[0].1,
                &format!("Persistent BindGroup A {}", pass_index)
            );
            let bind_group_b = self.create_texture_bind_group(
                device,
                layout,
                &textures[1].1,
                &format!("Persistent BindGroup B {}", pass_index)
            );

            self.persistent_bind_groups.insert(pass_index, [bind_group_a, bind_group_b]);
        }

        // Return bind group for reading (previous frame)
        let bind_groups = self.persistent_bind_groups.get(&pass_index).unwrap();
        let read_index = frame_buffer::previous_buffer_index(self.current_frame);
        &bind_groups[read_index]
    }

    /// Strategy implementation for intermediate texture creation
    fn create_intermediate_texture_strategy(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        if !self.intermediate_textures.contains_key(&pass_index) {
            console_log!("📝 Creating intermediate texture for pass {}", pass_index);
            let (texture, view) =
                self.create_texture(device, &format!("WASM Intermediate Texture {}", pass_index));
            self.intermediate_textures.insert(pass_index, (texture, view));
        }
        let (_, view) = self.intermediate_textures.get(&pass_index).unwrap();
        (view, &self.sampler)
    }

    /// Strategy implementation for ping-pong texture creation
    fn create_ping_pong_texture_strategy(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        if !self.ping_pong_textures.contains_key(&pass_index) {
            console_log!("🏓 Creating ping-pong textures for pass {}", pass_index);
            let (texture_a, view_a) =
                self.create_texture(device, &format!("WASM Ping Pong Texture A {}", pass_index));
            let (texture_b, view_b) =
                self.create_texture(device, &format!("WASM Ping Pong Texture B {}", pass_index));
            self.ping_pong_textures
                .insert(pass_index, [(texture_a, view_a), (texture_b, view_b)]);
            self.ping_pong_initialized.insert(pass_index, false);
        }

        // Return previous frame texture for reading
        let textures = self.ping_pong_textures.get(&pass_index).unwrap();
        let read_index = frame_buffer::previous_buffer_index(self.current_frame);
        let (_, view) = &textures[read_index];
        (view, &self.sampler)
    }

    /// Strategy implementation for persistent texture creation
    fn create_persistent_texture_strategy(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        if !self.persistent_textures.contains_key(&pass_index) {
            console_log!("💾 Creating persistent textures for pass {}", pass_index);
            let (texture_a, view_a) =
                self.create_texture(device, &format!("WASM Persistent Texture A {}", pass_index));
            let (texture_b, view_b) =
                self.create_texture(device, &format!("WASM Persistent Texture B {}", pass_index));
            self.persistent_textures
                .insert(pass_index, [(texture_a, view_a), (texture_b, view_b)]);
            self.persistent_initialized.insert(pass_index, false);
        }

        // Return previous frame texture for reading
        let textures = self.persistent_textures.get(&pass_index).unwrap();
        let read_index = frame_buffer::previous_buffer_index(self.current_frame);
        let (_, view) = &textures[read_index];
        (view, &self.sampler)
    }

    /// Get or create intermediate texture
    pub fn get_or_create_intermediate_texture(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        self.get_or_create_texture(device, WasmTextureType::Intermediate, pass_index)
    }

    /// Get or create ping-pong texture
    pub fn get_or_create_ping_pong_texture(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        self.get_or_create_texture(device, WasmTextureType::PingPong, pass_index)
    }

    /// Get or create persistent texture
    pub fn get_or_create_persistent_texture(
        &mut self,
        device: &Device,
        pass_index: usize,
    ) -> (&TextureView, &Sampler) {
        self.get_or_create_texture(device, WasmTextureType::Persistent, pass_index)
    }

    /// Check if persistent texture is initialized
    pub fn is_persistent_texture_initialized(&self, pass_index: usize) -> bool {
        self.persistent_initialized
            .get(&pass_index)
            .copied()
            .unwrap_or(false)
    }

    /// Mark persistent texture as initialized
    pub fn mark_persistent_texture_initialized(&mut self, pass_index: usize) {
        console_log!("✅ Marking persistent texture {} as initialized", pass_index);
        self.persistent_initialized.insert(pass_index, true);
    }

    /// Check if ping-pong texture is initialized
    pub fn is_ping_pong_texture_initialized(&self, pass_index: usize) -> bool {
        self.ping_pong_initialized
            .get(&pass_index)
            .copied()
            .unwrap_or(false)
    }

    /// Mark ping-pong texture as initialized
    pub fn mark_ping_pong_texture_initialized(&mut self, pass_index: usize) {
        console_log!("✅ Marking ping-pong texture {} as initialized", pass_index);
        self.ping_pong_initialized.insert(pass_index, true);
    }

    /// Get render target for ping-pong textures (write target)
    pub fn get_ping_pong_render_target(&self, pass_index: usize) -> Option<&TextureView> {
        self.ping_pong_textures.get(&pass_index).map(|textures| {
            let write_index = frame_buffer::current_buffer_index(self.current_frame);
            &textures[write_index].1
        })
    }

    /// Get render target for intermediate textures
    pub fn get_intermediate_render_target(&self, pass_index: usize) -> Option<&TextureView> {
        self.intermediate_textures
            .get(&pass_index)
            .map(|(_, view)| view)
    }

    /// Get render target for persistent textures (write target)
    pub fn get_persistent_render_target(&self, pass_index: usize) -> Option<&TextureView> {
        self.persistent_textures.get(&pass_index).map(|textures| {
            let write_index = frame_buffer::current_buffer_index(self.current_frame);
            &textures[write_index].1
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_texture_type_equality() {
        assert_eq!(WasmTextureType::Intermediate, WasmTextureType::Intermediate);
        assert_eq!(WasmTextureType::PingPong, WasmTextureType::PingPong);
        assert_eq!(WasmTextureType::Persistent, WasmTextureType::Persistent);

        assert_ne!(WasmTextureType::Intermediate, WasmTextureType::PingPong);
    }

    #[test]
    fn test_frame_calculations() {
        // These should match the frame_buffer calculations
        let current_0 = frame_buffer::current_buffer_index(0);
        let previous_0 = frame_buffer::previous_buffer_index(0);

        assert_eq!(current_0, 0);
        assert_eq!(previous_0, 1);

        let current_1 = frame_buffer::current_buffer_index(1);
        let previous_1 = frame_buffer::previous_buffer_index(1);

        assert_eq!(current_1, 1);
        assert_eq!(previous_1, 0);
    }
}