//! Shader rendering system for shekere
//!
//! This module provides Material2d-based rendering with dynamic shader loading,
//! multi-pass rendering, and persistent (trail) effects.

mod generation;
mod materials;
mod setup;
mod state;

use bevy::prelude::*;
use bevy::sprite::Material2dPlugin;
use materials::{ShekereShaderMaterial, ShekereShaderMaterialPass0, ShekereShaderMaterialPass1};
use setup::setup_dynamic_shader_system;
use state::{MultiPassState, PersistentPassState};

// Re-export for internal use only
pub(crate) use generation::{
    check_multipass_shader_reload, check_persistent_shader_reload, check_shader_reload,
};
pub(crate) use setup::{
    update_multipass_uniforms, update_persistent_uniforms, update_shader_uniforms,
};
pub use state::InputBufferHandles;

/// Plugin for shader rendering with dynamic loading and multi-pass support
pub struct ShaderRenderPlugin;

impl Plugin for ShaderRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Material2dPlugin::<ShekereShaderMaterial>::default(),
            Material2dPlugin::<ShekereShaderMaterialPass0>::default(),
            Material2dPlugin::<ShekereShaderMaterialPass1>::default(),
        ))
        .add_systems(Startup, setup_dynamic_shader_system)
        .add_systems(
            Update,
            (
                update_shader_uniforms,
                update_multipass_uniforms.run_if(resource_exists::<MultiPassState>),
                update_persistent_uniforms.run_if(resource_exists::<PersistentPassState>),
                check_shader_reload,
                check_multipass_shader_reload.run_if(resource_exists::<MultiPassState>),
                check_persistent_shader_reload.run_if(resource_exists::<PersistentPassState>),
            ),
        );
    }
}
