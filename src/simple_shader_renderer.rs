// WGSL shader rendering using Bevy's material system
// This integrates actual WGSL shaders from configuration files

use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::sprite::{Material2d, Material2dPlugin};
use std::fs;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use crate::shader_preprocessor::ShaderPreprocessor;
use bytemuck::{Pod, Zeroable};

// Mouse data structures for GPU
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
struct MouseShaderData {
    position: [f32; 2],
    _padding: [f32; 2], // vec4 alignment
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
struct MouseHistoryBuffer {
    // 512 frames of mouse history data
    history_data: [MouseShaderData; 512],
}

// Resource to track mouse history locally
#[derive(Resource)]
struct MouseHistoryTracker {
    history: [MouseShaderData; 512],
    current_index: usize,
    last_mouse_pos: [f32; 2],
}

impl Default for MouseHistoryTracker {
    fn default() -> Self {
        Self {
            history: [MouseShaderData {
                position: [0.0, 0.0],
                _padding: [0.0, 0.0],
            }; 512],
            current_index: 0,
            last_mouse_pos: [0.0, 0.0],
        }
    }
}

// Plugin for simple shader rendering
pub struct SimpleShaderRenderPlugin;

impl Plugin for SimpleShaderRenderPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(Material2dPlugin::<ShekerShaderMaterial>::default())
            .init_resource::<MouseHistoryTracker>()
            .add_systems(Startup, setup_dynamic_shader_system)
            .add_systems(Update, (update_shader_uniforms, update_mouse_history, check_shader_reload));
    }
}

// Constant handle for our dynamic shader
const DYNAMIC_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(0x9E4B8A2F1C6D3E7F8A9B4C5D6E7F8A9B);

// Resource to hold dynamic shader state
#[derive(Resource)]
struct DynamicShaderState {
    last_config_hash: u64,
}

// Custom material for loading WGSL shaders
#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct ShekerShaderMaterial {
    #[uniform(0)]
    resolution: Vec2,
    #[uniform(1)]
    duration: f32,
    #[storage(2, read_only)]
    mouse_history: Handle<ShaderStorageBuffer>,
}

impl Material2d for ShekerShaderMaterial {
    fn fragment_shader() -> ShaderRef {
        // Always return our fixed dynamic shader handle
        DYNAMIC_SHADER_HANDLE.into()
    }
}

// Component to mark our fullscreen quad
#[derive(Component)]
struct FullscreenQuad;

// Setup dynamic shader system
fn setup_dynamic_shader_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ShekerShaderMaterial>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    config: Res<crate::ShekerConfig>,
    windows: Query<&Window>,
) {
    log::info!("Setting up dynamic WGSL shader rendering with Assets<Shader>");

    let Ok(window) = windows.get_single() else {
        log::error!("Could not get window for shader setup");
        return;
    };

    log::info!("Window found: {}x{}", window.width(), window.height());

    log::info!("About to generate shader source...");

    // Generate shader source using ShaderPreprocessor
    let shader_source = match generate_clean_shader_source(&config) {
        Ok(source) => {
            log::info!("Successfully generated shader source ({} chars)", source.len());
            source
        }
        Err(e) => {
            log::error!("Failed to generate shader source: {}. Using fallback shader.", e);
            // Use a simple fallback shader with common.wgsl and animated colors
            let common_wgsl = include_str!("../shaders/common.wgsl");
            format!(r#"#import bevy_sprite::mesh2d_vertex_output::VertexOutput

{}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {{
    let normalized_coords = NormalizedCoords(mesh.position.xy);
    let color = vec3(
        sin(Time.duration + normalized_coords.x) * 0.5 + 0.5,
        cos(Time.duration + normalized_coords.y) * 0.5 + 0.5,
        sin(Time.duration + length(normalized_coords)) * 0.5 + 0.5
    );
    return vec4(ToLinearRgb(color), 1.0);
}}
"#, common_wgsl)
        }
    };

    log::info!("Using shader source ({} chars)", shader_source.len());

    log::info!("About to create shader asset...");

    // Create shader asset directly in Assets<Shader> with our fixed handle
    let shader = Shader::from_wgsl(shader_source, "dynamic_shader.wgsl");
    shaders.insert(&DYNAMIC_SHADER_HANDLE, shader);

    log::info!("Created dynamic shader with handle: {:?}", DYNAMIC_SHADER_HANDLE);

    // Calculate config hash for change detection
    let config_hash = calculate_config_hash(&config);
    log::info!("Calculated config hash: {}", config_hash);

    // Initialize dynamic shader state
    commands.insert_resource(DynamicShaderState {
        last_config_hash: config_hash,
    });

    log::info!("Dynamic shader state initialized");

    log::info!("Creating fullscreen quad mesh...");

    // Create a fullscreen quad mesh
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    // Fullscreen quad vertices covering the entire screen
    let vertices = vec![
        [-1.0, -1.0, 0.0], // bottom left
        [1.0, -1.0, 0.0],  // bottom right
        [1.0, 1.0, 0.0],   // top right
        [-1.0, 1.0, 0.0],  // top left
    ];

    // UV coordinates for the quad
    let uvs = vec![
        [0.0, 1.0], // bottom left
        [1.0, 1.0], // bottom right
        [1.0, 0.0], // top right
        [0.0, 0.0], // top left
    ];

    let indices = vec![0u32, 1, 2, 2, 3, 0];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    log::info!("Mesh created successfully");

    // Create the WGSL shader material
    // Initialize mouse history buffer with zeros
    let mouse_history_data = MouseHistoryBuffer {
        history_data: [MouseShaderData {
            position: [0.0, 0.0],
            _padding: [0.0, 0.0],
        }; 512],
    };

    // Create storage buffer asset
    let mouse_buffer_handle = storage_buffers.add(ShaderStorageBuffer::from(mouse_history_data));

    let material = materials.add(ShekerShaderMaterial {
        resolution: Vec2::new(window.width(), window.height()),
        duration: 0.0,
        mouse_history: mouse_buffer_handle,
    });

    // Spawn the fullscreen quad using Bevy's 2D coordinate system
    // Scale to cover the entire screen in Bevy 2D coordinates
    commands.spawn((
        Mesh2d(meshes.add(mesh)),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(window.width(), window.height(), 1.0)),
        FullscreenQuad,
    ));

    log::info!("Spawned fullscreen quad with material and scale {}x{}", window.width(), window.height());

    // Add a standard 2D camera
    commands.spawn(Camera2d);

    log::info!("Spawned standard 2D camera");

    log::info!("=== Dynamic WGSL shader rendering setup completed successfully ===");
}

// Update shader uniforms every frame
fn update_shader_uniforms(
    time: Res<Time>,
    windows: Query<&Window>,
    mut materials: ResMut<Assets<ShekerShaderMaterial>>,
    quad_query: Query<&MeshMaterial2d<ShekerShaderMaterial>, With<FullscreenQuad>>,
) {
    let Ok(window) = windows.get_single() else {
        return; // Skip update if no window
    };
    let elapsed = time.elapsed_secs();

    let mut updated_count = 0;
    for material_handle in quad_query.iter() {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.resolution = Vec2::new(window.width(), window.height());
            material.duration = elapsed;
            updated_count += 1;
        }
    }

    // Log every 60 frames (approximately once per second at 60fps)
    if (elapsed * 60.0) as u32 % 60 == 0 && updated_count > 0 {
        log::info!("Updated {} materials - time: {:.2}s", updated_count, elapsed);
    }
}

// Update mouse history in the materials
fn update_mouse_history(
    windows: Query<&Window>,
    materials: Res<Assets<ShekerShaderMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut mouse_tracker: ResMut<MouseHistoryTracker>,
    quad_query: Query<&MeshMaterial2d<ShekerShaderMaterial>, With<FullscreenQuad>>,
) {
    let Ok(window) = windows.get_single() else {
        return; // Skip update if no window
    };

    // Get mouse position (if available)
    let mouse_pos = if let Some(cursor_pos) = window.cursor_position() {
        [cursor_pos.x, cursor_pos.y]
    } else {
        // Use last known position if cursor not in window
        mouse_tracker.last_mouse_pos
    };

    // Check if mouse has moved significantly (threshold to avoid jitter)
    let threshold = 1.0; // pixels
    let mouse_moved =
        (mouse_pos[0] - mouse_tracker.last_mouse_pos[0]).abs() > threshold ||
        (mouse_pos[1] - mouse_tracker.last_mouse_pos[1]).abs() > threshold;

    // Always update the current position (index 0) for smooth animation
    mouse_tracker.history[0] = MouseShaderData {
        position: mouse_pos,
        _padding: [0.0, 0.0],
    };

    // Only shift history when mouse moves or every few frames for smooth trail fading
    if mouse_moved {
        mouse_tracker.last_mouse_pos = mouse_pos;

        // Shift history: move all entries one position back
        for i in (1..512).rev() {
            mouse_tracker.history[i] = mouse_tracker.history[i - 1];
        }
    }

    // Create GPU buffer data from our tracked history
    let buffer_data = MouseHistoryBuffer {
        history_data: mouse_tracker.history,
    };

    // Update storage buffers for all materials
    for material_handle in quad_query.iter() {
        if let Some(material) = materials.get(&material_handle.0) {
            if let Some(buffer) = storage_buffers.get_mut(&material.mouse_history) {
                buffer.set_data(buffer_data);
            }
        }
    }
}

// Generate dynamic shader file using ShaderPreprocessor
fn generate_dynamic_shader_file(config: &crate::ShekerConfig) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Generating dynamic shader file using ShaderPreprocessor");

    // Clean up old dynamic shaders
    let assets_dir = "assets/shaders";
    if let Ok(entries) = fs::read_dir(assets_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(filename) = entry.file_name().to_str() {
                    if filename.starts_with("dynamic_shader") && filename.ends_with(".wgsl") {
                        let _ = fs::remove_file(entry.path());
                        log::info!("Removed old shader file: {}", filename);
                    }
                }
            }
        }
    }

    // Use ShaderPreprocessor to generate the shader
    let preprocessor = ShaderPreprocessor::new(&config.config_dir);
    let fragment_path = config.config_dir.join(&config.config.pipeline[0].file);

    let mut processed_shader = preprocessor
        .process_file_with_embedded_defs_and_multipass(&fragment_path, false)
        .map_err(|e| format!("Failed to process shader: {:?}", e))?;

    // Keep Time.duration as-is for compatibility with existing shaders
    // processed_shader = processed_shader.replace("Time.duration", "Time.time");

    // Remove conflicting and unused bindings for basic shader compatibility
    let lines: Vec<&str> = processed_shader.lines().collect();
    let mut filtered_lines = Vec::new();
    let mut skip_lines = false;
    let mut skip_function = false;

    for line in lines {
        let trimmed = line.trim();

        // Skip Group 0 binding declarations that conflict with our Group 2 bindings
        if trimmed.starts_with("@group(0) @binding(0) var<uniform> Window") ||
           trimmed.starts_with("@group(0) @binding(1) var<uniform> Time") {
            continue;
        }

        // Skip Group 1 and Group 0 bindings that we don't provide in basic mode
        if trimmed.starts_with("@group(1)") ||
           (trimmed.starts_with("@group(0)") &&
            (trimmed.contains("Osc") || trimmed.contains("Spectrum") || trimmed.contains("Midi"))) {
            continue;
        }

        // Skip duplicate struct definitions (we define them in Bevy wrapper)
        if trimmed.starts_with("struct WindowUniform") ||
           trimmed.starts_with("struct TimeUniform") {
            skip_lines = true;
            continue;
        }

        // Skip functions that use unavailable resources
        if trimmed.contains("fn Spectrum") || trimmed.contains("fn Osc") || trimmed.contains("fn Midi") ||
           trimmed.contains("fn Mouse") {
            skip_function = true;
            continue;
        }

        // End of struct or function definition
        if (skip_lines || skip_function) && trimmed == "}" {
            skip_lines = false;
            skip_function = false;
            continue;
        }

        // Skip lines inside struct or function
        if skip_lines || skip_function {
            continue;
        }

        filtered_lines.push(line);
    }

    processed_shader = filtered_lines.join("\n");

    // Load common.wgsl with all definitions
    let common_wgsl = include_str!("../shaders/common.wgsl");

    // Create Bevy-compatible shader with proper imports and structure
    let combined_shader = format!(
        r#"#import bevy_sprite::mesh2d_vertex_output::VertexOutput

{}

// === PROCESSED SHADER CODE ===

{processed_shader}

// === BEVY FRAGMENT ENTRY POINT ===

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {{
    // Set up vertex output for compatibility
    var in: VertexOutput;
    in.position = mesh.position;
    in.uv = mesh.uv;

    // Map to expected coordinate system
    let tex_coords = mesh.uv;

    // Call the fs_main function from processed shader
    return fs_main(in, tex_coords);
}}
"#,
        common_wgsl,
        processed_shader = processed_shader
    );

    // Write to fixed filename that Bevy can find
    let output_path = "assets/shaders/dynamic_shader.wgsl";

    fs::create_dir_all("assets/shaders")?;
    let mut file = fs::File::create(&output_path)?;
    file.write_all(combined_shader.as_bytes())?;

    log::info!("Dynamic shader file generated successfully at {}", output_path);
    Ok(())
}

// Generate shader using ShaderPreprocessor
fn generate_shader_with_preprocessor(config: &crate::ShekerConfig) -> Result<String, Box<dyn std::error::Error>> {
    log::info!("Generating shader with ShaderPreprocessor");

    // Create shader preprocessor
    let preprocessor = ShaderPreprocessor::new(&config.config_dir);

    // Get fragment shader path from config
    let fragment_path = config.config_dir.join(&config.config.pipeline[0].file);

    // Process the shader with embedded definitions
    let shader_source = preprocessor
        .process_file_with_embedded_defs_and_multipass(&fragment_path, false)
        .map_err(|e| format!("Failed to process shader: {:?}", e))?;

    log::info!("Shader generated successfully with {} characters", shader_source.len());
    Ok(shader_source)
}

// Calculate hash of configuration for change detection
fn calculate_config_hash(config: &crate::ShekerConfig) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Hash the pipeline configuration
    for pipeline in &config.config.pipeline {
        pipeline.file.hash(&mut hasher);
        pipeline.label.hash(&mut hasher);
        pipeline.entry_point.hash(&mut hasher);
        pipeline.ping_pong.hash(&mut hasher);
        pipeline.persistent.hash(&mut hasher);
    }

    hasher.finish()
}

// System to check for shader reload
fn check_shader_reload(
    config: Res<crate::ShekerConfig>,
    mut shader_state: Option<ResMut<DynamicShaderState>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    let Some(mut state) = shader_state else {
        return;
    };

    let current_hash = calculate_config_hash(&config);

    // Check if configuration has changed
    if current_hash != state.last_config_hash {
        log::info!("Configuration changed, reloading shader dynamically");

        // Generate new clean shader source
        match generate_clean_shader_source(&config) {
            Ok(new_shader_source) => {
                // Update the existing shader asset directly
                let new_shader = Shader::from_wgsl(new_shader_source, "dynamic_shader.wgsl");
                shaders.insert(&DYNAMIC_SHADER_HANDLE, new_shader);

                // Update state
                state.last_config_hash = current_hash;

                log::info!("Shader updated dynamically in Assets<Shader>");
            }
            Err(e) => {
                log::error!("Failed to reload shader: {}", e);
            }
        }
    }
}

// Generate clean shader source with modular WGSL inclusion
fn generate_clean_shader_source(config: &crate::ShekerConfig) -> Result<String, Box<dyn std::error::Error>> {
    log::info!("Generating shader with modular WGSL inclusion for Material2d rendering");

    // Read the fragment shader file directly
    let fragment_path = config.config_dir.join(&config.config.pipeline[0].file);
    let fragment_source = std::fs::read_to_string(&fragment_path)
        .map_err(|e| format!("Failed to read fragment shader: {:?}", e))?;

    // Check which features the shader uses
    let uses_mouse = fragment_source.contains("MouseCoords");
    let uses_osc = fragment_source.contains("Osc");
    let uses_spectrum = fragment_source.contains("Spectrum");
    let uses_midi = fragment_source.contains("Midi");
    let uses_texture = fragment_source.contains("SamplePreviousPass") || fragment_source.contains("previous_pass");

    // Start with Bevy import
    let mut shader_parts = vec![
        "#import bevy_sprite::mesh2d_vertex_output::VertexOutput".to_string(),
        "".to_string(),
    ];

    // Always include core definitions
    let core_wgsl = include_str!("../shaders/core.wgsl");
    shader_parts.push("// === CORE DEFINITIONS ===".to_string());
    shader_parts.push(core_wgsl.to_string());
    shader_parts.push("".to_string());

    // Add conditional features only if used
    if uses_mouse {
        let mouse_wgsl = include_str!("../shaders/mouse.wgsl");
        shader_parts.push("// === MOUSE DEFINITIONS ===".to_string());
        shader_parts.push(mouse_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including mouse input module");
    }

    if uses_spectrum {
        let spectrum_wgsl = include_str!("../shaders/spectrum.wgsl");
        shader_parts.push("// === SPECTRUM DEFINITIONS ===".to_string());
        shader_parts.push(spectrum_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including spectrum analysis module");
    }

    if uses_osc {
        let osc_wgsl = include_str!("../shaders/osc.wgsl");
        shader_parts.push("// === OSC DEFINITIONS ===".to_string());
        shader_parts.push(osc_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including OSC input module");
    }

    if uses_midi {
        let midi_wgsl = include_str!("../shaders/midi.wgsl");
        shader_parts.push("// === MIDI DEFINITIONS ===".to_string());
        shader_parts.push(midi_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including MIDI input module");
    }

    if uses_texture {
        let texture_wgsl = include_str!("../shaders/texture.wgsl");
        shader_parts.push("// === TEXTURE DEFINITIONS ===".to_string());
        shader_parts.push(texture_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including multi-pass texture module");
    }

    // Add user fragment shader
    shader_parts.push("// === USER FRAGMENT SHADER ===".to_string());
    shader_parts.push(fragment_source.replace("fn fs_main(", "fn fragment("));

    let final_shader = shader_parts.join("\n");

    log::info!("Generated minimal shader with {} characters", final_shader.len());

    // DEBUG: Write shader to file for inspection
    if let Err(e) = std::fs::write("/tmp/bevy_shader.wgsl", &final_shader) {
        log::warn!("Failed to write debug shader: {}", e);
    } else {
        log::info!("Debug shader written to /tmp/bevy_shader.wgsl");
    }

    // DEBUG: Output first 1000 characters of generated shader
    let preview = if final_shader.len() > 1000 {
        &final_shader[..1000]
    } else {
        &final_shader
    };
    log::info!("Generated shader preview:\n{}", preview);

    Ok(final_shader)
}