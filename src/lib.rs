// mod basic_shader_renderer; // Temporarily disabled due to Bevy 0.16.1 API changes
mod bevy_inputs;
mod bevy_rendering;
mod bind_group_factory;
pub mod config;
pub mod hot_reload;
mod inputs;
pub mod pipeline;
pub mod render_constants;
mod shader_preprocessor;
mod simple_shader_renderer;
mod state;
pub mod texture_manager;
mod timer;
mod uniforms;
mod vertex;

pub use crate::config::Config;
pub use crate::config::ShaderConfig;
pub use crate::state::State;

// Bevy-related structures are defined below and automatically exported
use bevy::prelude::*;
use std::path::PathBuf;

// Bevy resource to hold configuration and config directory
#[derive(Resource)]
pub struct ShekerConfig {
    pub config: Config,
    pub config_dir: PathBuf,
}

// Import required types for resources
use crate::bevy_inputs::{
    midi_input_system, mouse_input_system, osc_input_system, setup_midi_input_system,
    setup_mouse_input_system, setup_osc_input_system, setup_spectrum_input_system,
    spectrum_input_system,
};
use crate::hot_reload::HotReloader;
use crate::pipeline::MultiPassPipeline;
use crate::simple_shader_renderer::SimpleShaderRenderPlugin;
use crate::texture_manager::TextureManager;
use crate::timer::Timer;
use crate::uniforms::time_uniform::TimeUniform;
use crate::uniforms::window_uniform::WindowUniform;

// Bevy resource to replace State struct fields
#[derive(Resource)]
pub struct ShekerState {
    pub multi_pass_pipeline: MultiPassPipeline,
    pub texture_manager: TextureManager,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub timer: Timer,
    pub hot_reloader: Option<HotReloader>,
    pub uniform_bind_group: wgpu::BindGroup,
    pub device_bind_group: wgpu::BindGroup,
    pub sound_bind_group: Option<wgpu::BindGroup>,
}

// TODO: Input managers need special handling for thread safety
// Will be implemented in separate migration steps
// #[derive(Resource)]
// pub struct InputManagers {
//     pub spectrum: Option<SpectrumInputManager>,
//     pub midi: Option<MidiInputManager>,
//     pub mouse: Option<MouseInputManager>,
//     pub osc: Option<OscInputManager<'static>>,
// }

// Bevy resource for uniforms
#[derive(Resource)]
pub struct UniformBuffers {
    pub window_uniform: WindowUniform,
    pub time_uniform: TimeUniform,
}

// Main Bevy plugin for shekere
pub struct ShekerPlugin;

impl Plugin for ShekerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimpleShaderRenderPlugin)
            .add_systems(
                Startup,
                (
                    setup_sheker_state,
                    setup_midi_input_system,
                    setup_mouse_input_system,
                    setup_osc_input_system,
                    setup_spectrum_input_system,
                    debug_startup_system,
                ),
            )
            .add_systems(
                Update,
                (
                    midi_input_system,
                    mouse_input_system,
                    osc_input_system,
                    spectrum_input_system,
                    update_uniforms,
                    hot_reload_system,
                ),
            );
    }
}

// Debug system to verify basic Bevy functionality
fn debug_startup_system() {
    log::info!("=== Sheker debug startup system executed ===");
    println!("Debug: Sheker plugin initialization completed");
}

// Initialize sheker state with Bevy context - replaces State::new() functionality
fn setup_sheker_state(
    _commands: Commands,
    config: Res<ShekerConfig>,
    // TODO: Add RenderDevice and RenderQueue when integrating with Bevy rendering
) {
    log::info!(
        "Setting up sheker state with config: {:?}",
        config.config.window
    );

    // TODO: Initialize ShekerState resource
    // This will require integration with Bevy's rendering system
    // For now, we'll add the basic structure

    // TODO: Initialize UniformBuffers resource
    // commands.insert_resource(UniformBuffers {
    //     window_uniform: WindowUniform::new(),
    //     time_uniform: TimeUniform::new(),
    // });

    log::info!("Sheker state setup placeholder completed");
}

// Individual input systems are now implemented in bevy_inputs module

fn update_uniforms() {
    // Uniform updates will be implemented here
    // This will replace state.update() functionality
}

fn hot_reload_system(_config: Res<ShekerConfig>) {
    // Hot reload functionality will be implemented here
}

// Rendering is now handled by ShekerRenderPlugin in bevy_rendering module
