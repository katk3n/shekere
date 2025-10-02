pub mod config;
pub mod hot_reload;
pub mod inputs;
mod shader_renderer;

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
use crate::shader_renderer::ShaderRenderPlugin;

// Main Bevy plugin for shekere
pub struct ShekerePlugin;

impl Plugin for ShekerePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ShaderRenderPlugin)
            // Input systems must run after ShaderRenderPlugin setup in PostStartup
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
                ),
            );
    }
}
