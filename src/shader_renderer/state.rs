//! Rendering state management structures

use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;

/// Resource containing handles to storage buffers used by input managers
#[derive(Resource, Clone)]
pub struct InputBufferHandles {
    pub mouse_buffer: Handle<ShaderStorageBuffer>,
    pub midi_buffer: Handle<ShaderStorageBuffer>,
    pub spectrum_buffer: Handle<ShaderStorageBuffer>,
    pub osc_buffer: Handle<ShaderStorageBuffer>,
}

/// Resource to hold dynamic shader state
#[derive(Resource)]
pub(super) struct DynamicShaderState {
    pub last_config_hash: u64,
}

/// Resource to track multi-pass rendering state
#[derive(Resource)]
#[allow(dead_code)]
pub(super) struct MultiPassState {
    pub pass_count: usize,
    pub intermediate_textures: Vec<Handle<Image>>,
    pub pass_shader_handles: Vec<Handle<Shader>>,
    pub pass_entities: Vec<Entity>,
}

/// Resource to track persistent texture rendering state (trail effects)
#[derive(Resource)]
pub(super) struct PersistentPassState {
    pub frame_count: u64,
    pub textures: [Handle<Image>; 2], // Double-buffered textures for ping-pong
    pub entity: Entity,               // Trail rendering entity
    pub camera_a: Entity,             // Camera A: renders to texture_a
    pub camera_b: Entity,             // Camera B: renders to texture_b
    pub display_entity: Entity,       // Display entity showing the result
}

/// Component to mark render pass entities
#[derive(Component)]
#[allow(dead_code)]
pub(super) struct RenderPassMarker {
    pub pass_index: usize,
    pub is_final_pass: bool,
}
