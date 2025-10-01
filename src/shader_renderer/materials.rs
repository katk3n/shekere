//! Bevy Material2d implementations for shader rendering

use bevy::asset::weak_handle;
use bevy::prelude::*;
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::sprite::{Material2d, Material2dKey};

// Shader handles
pub(super) const DYNAMIC_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("9e4b8a2f-1c6d-4e7f-8a9b-4c5d6e7f8a9b");
pub(super) const PASS_0_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("9e4b8a2f-1c6d-4e7f-8a9b-4c5d6e7f8a9c");
pub(super) const PASS_1_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("9e4b8a2f-1c6d-4e7f-8a9b-4c5d6e7f8a9d");

/// Custom material for loading WGSL shaders (main/single-pass)
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct ShekereShaderMaterial {
    #[uniform(0)]
    pub resolution: Vec2,
    #[uniform(1)]
    pub duration: f32,
    #[storage(2, read_only)]
    pub mouse_history: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)]
    pub spectrum_history: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)]
    pub osc_history: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)]
    pub midi_history: Handle<ShaderStorageBuffer>,
    // Multi-pass texture bindings (optional - only used in multi-pass rendering)
    #[texture(6)]
    #[sampler(7)]
    pub previous_pass_texture: Option<Handle<Image>>,
}

impl Material2d for ShekereShaderMaterial {
    fn fragment_shader() -> ShaderRef {
        // Always return our fixed dynamic shader handle
        DYNAMIC_SHADER_HANDLE.into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set custom entry point name to match generated shader
        // Note: generate_clean_shader_source replaces "fs_main" with "fragment"
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = "fragment".into();
        }
        Ok(())
    }
}

// Pass-specific material types for multi-pass rendering
// Each pass needs its own Material type because Material2d::fragment_shader() is static

/// Material for pass 0 in multi-pass rendering
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct ShekereShaderMaterialPass0 {
    #[uniform(0)]
    pub resolution: Vec2,
    #[uniform(1)]
    pub duration: f32,
    #[storage(2, read_only)]
    pub mouse_history: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)]
    pub spectrum_history: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)]
    pub osc_history: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)]
    pub midi_history: Handle<ShaderStorageBuffer>,
    #[texture(6)]
    #[sampler(7)]
    pub previous_pass_texture: Option<Handle<Image>>,
}

impl Material2d for ShekereShaderMaterialPass0 {
    fn fragment_shader() -> ShaderRef {
        PASS_0_SHADER_HANDLE.into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set custom entry point name to match generated shader
        // Note: generate_shader_for_pass does NOT replace "fs_main"
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = "fs_main".into();
        }
        Ok(())
    }
}

/// Material for pass 1 in multi-pass rendering
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(super) struct ShekereShaderMaterialPass1 {
    #[uniform(0)]
    pub resolution: Vec2,
    #[uniform(1)]
    pub duration: f32,
    #[storage(2, read_only)]
    pub mouse_history: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)]
    pub spectrum_history: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)]
    pub osc_history: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)]
    pub midi_history: Handle<ShaderStorageBuffer>,
    #[texture(6)]
    #[sampler(7)]
    pub previous_pass_texture: Option<Handle<Image>>,
}

impl Material2d for ShekereShaderMaterialPass1 {
    fn fragment_shader() -> ShaderRef {
        PASS_1_SHADER_HANDLE.into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set custom entry point name to match generated shader
        // Note: generate_shader_for_pass does NOT replace "fs_main"
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = "fs_main".into()
        }
        Ok(())
    }
}

/// Component to mark our fullscreen quad
#[derive(Component)]
pub(super) struct FullscreenQuad;
