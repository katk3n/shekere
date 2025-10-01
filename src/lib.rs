pub mod config;
pub mod hot_reload;
pub mod inputs;
mod shader_renderer;
mod timer;

pub use crate::config::Config;
pub use crate::config::ShaderConfig;

// Bevy-related structures are defined below and automatically exported
use bevy::prelude::*;
use std::path::PathBuf;

// Bevy resource to hold configuration and config directory
#[derive(Resource)]
pub struct ShekereConfig {
    pub config: Config,
    pub config_dir: PathBuf,
}

// Import required types for resources
use crate::inputs::{
    midi_input_system, mouse_input_system, osc_input_system, setup_midi_input_system,
    setup_mouse_input_system, setup_osc_input_system, setup_spectrum_input_system,
    spectrum_input_system,
};
use crate::shader_renderer::SimpleShaderRenderPlugin;

// Main Bevy plugin for shekere
pub struct ShekerePlugin;

impl Plugin for ShekerePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimpleShaderRenderPlugin)
            .add_systems(Startup, (setup_shekere_state, debug_startup_system))
            // Input systems must run after SimpleShaderRenderPlugin setup in PostStartup
            .add_systems(
                bevy::app::PostStartup,
                (
                    setup_midi_input_system,
                    setup_mouse_input_system,
                    setup_osc_input_system,
                    setup_spectrum_input_system,
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
    log::info!("=== Shekere debug startup system executed ===");
    println!("Debug: Shekere plugin initialization completed");
}

// Initialize shekere state with Bevy context
fn setup_shekere_state(_commands: Commands, config: Res<ShekereConfig>) {
    log::info!(
        "Setting up shekere state with config: {:?}",
        config.config.window
    );
    log::info!("Shekere state setup placeholder completed");
}

fn update_uniforms() {
    // Uniform updates will be implemented here
}

fn hot_reload_system(_config: Res<ShekereConfig>) {
    // Hot reload functionality will be implemented here
}

// Rendering is now handled by SimpleShaderRenderPlugin
