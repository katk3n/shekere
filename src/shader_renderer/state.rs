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

/// Resource to hold hot reload state (for single-pass mode)
#[derive(Resource)]
pub(crate) struct HotReloaderResource {
    pub reloader: Option<crate::hot_reload::HotReloader>,
    /// Paths to shader files being monitored.
    ///
    /// Future uses:
    /// - File-specific reload: Reload only the changed shader (not entire pipeline)
    /// - Error reporting: Show which shader file caused compilation error
    /// - Dependency tracking: Manage included shader files
    /// - Hot reload UI: Display monitored files in GUI (Phase 2)
    #[allow(dead_code)]
    pub shader_paths: Vec<std::path::PathBuf>,
}

/// Resource to hold dynamic shader state
#[derive(Resource)]
pub(crate) struct DynamicShaderState {
    pub last_config_hash: u64,
}

/// Resource to track multi-pass rendering state
#[derive(Resource)]
pub(crate) struct MultiPassState {
    pub pass_count: usize,
    /// Handles to intermediate render textures for each pass.
    ///
    /// Future uses:
    /// - Debug visualization: Display intermediate textures on screen
    /// - Texture export: Save intermediate results to files
    /// - Dynamic pipeline: Add/remove passes at runtime
    /// - Inspector integration: Show textures in Bevy Inspector (Phase 2)
    #[allow(dead_code)]
    pub intermediate_textures: Vec<Handle<Image>>,
    pub pass_shader_handles: Vec<Handle<Shader>>,
    /// Entities for each rendering pass.
    ///
    /// Future uses:
    /// - Pass control: Enable/disable specific passes for debugging
    /// - Transform manipulation: Apply effects to individual passes
    /// - Performance profiling: Measure render time per pass
    /// - Entity inspection: Debug pass entities in Bevy Inspector (Phase 2)
    #[allow(dead_code)]
    pub pass_entities: Vec<Entity>,
    pub hot_reloader: Option<crate::hot_reload::HotReloader>,
    /// Paths to shader files for each pass.
    ///
    /// Future uses:
    /// - Per-pass reload: Reload individual pass shaders
    /// - Error context: Show which pass shader failed to compile
    /// - Dependency tracking: Manage shader includes per pass
    /// - Hot reload UI: Display per-pass monitoring status (Phase 2)
    #[allow(dead_code)]
    pub shader_paths: Vec<std::path::PathBuf>,
}

/// Resource to track persistent texture rendering state (trail effects)
#[derive(Resource)]
pub(crate) struct PersistentPassState {
    pub frame_count: u64,
    pub textures: [Handle<Image>; 2], // Double-buffered textures for ping-pong
    pub entity: Entity,               // Trail rendering entity
    pub camera_a: Entity,             // Camera A: renders to texture_a
    pub camera_b: Entity,             // Camera B: renders to texture_b
    pub display_entity: Entity,       // Display entity showing the result
    pub hot_reloader: Option<crate::hot_reload::HotReloader>,
    /// Path to the persistent shader file being monitored.
    ///
    /// Future uses:
    /// - Targeted reload: Reload only the persistent shader on change
    /// - Error reporting: Show shader file path in compilation errors
    /// - Shader switching: Dynamically switch between different trail effects
    /// - Hot reload UI: Display current shader file in GUI (Phase 2)
    #[allow(dead_code)]
    pub shader_path: std::path::PathBuf,
}

/// Component to mark render pass entities
#[derive(Component)]
pub(crate) struct RenderPassMarker {
    /// Index of this rendering pass (0-based).
    ///
    /// Future uses:
    /// - Bevy Inspector: Display pass number in entity hierarchy
    /// - Conditional rendering: Enable/disable specific passes
    /// - Performance profiling: Track render time per pass index
    /// - Debug visualization: Color-code passes in debug view (Phase 2)
    #[allow(dead_code)]
    pub pass_index: usize,
    /// Whether this is the final rendering pass.
    ///
    /// Future uses:
    /// - Pipeline control: Dynamically change which pass is final
    /// - Debug output: Only show final pass in certain debug modes
    /// - Optimization: Apply post-processing only to final pass
    /// - UI indication: Highlight final pass in GUI inspector (Phase 2)
    #[allow(dead_code)]
    pub is_final_pass: bool,
}
