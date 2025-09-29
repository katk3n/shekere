// Bevy-compatible rendering system implementation
// This module integrates basic shader rendering with Bevy

use bevy::prelude::*;
use crate::simple_shader_renderer::SimpleShaderRenderPlugin;

// Main rendering plugin that orchestrates shader rendering
pub struct ShekerRenderPlugin;

impl Plugin for ShekerRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimpleShaderRenderPlugin);
    }
}

// Test that the plugin can be created
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let _plugin = ShekerRenderPlugin;
        // Basic test that the plugin structure exists
    }
}