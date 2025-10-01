//! Shader generation and hot-reload functionality

use super::materials::DYNAMIC_SHADER_HANDLE;
use super::state::DynamicShaderState;
use crate::shader_preprocessor::ShaderPreprocessor;
use bevy::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;

fn generate_dynamic_shader_file(
    config: &crate::ShekereConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Generating dynamic shader file using ShaderPreprocessor");

    // Clean up old dynamic shaders
    let assets_dir = "assets/shaders";
    if let Ok(entries) = fs::read_dir(assets_dir) {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.starts_with("dynamic_shader") && filename.ends_with(".wgsl") {
                    let _ = fs::remove_file(entry.path());
                    log::info!("Removed old shader file: {}", filename);
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
        if trimmed.starts_with("@group(0) @binding(0) var<uniform> Window")
            || trimmed.starts_with("@group(0) @binding(1) var<uniform> Time")
        {
            continue;
        }

        // Skip Group 1 and Group 0 bindings that we don't provide in basic mode
        if trimmed.starts_with("@group(1)")
            || (trimmed.starts_with("@group(0)")
                && (trimmed.contains("Osc")
                    || trimmed.contains("Spectrum")
                    || trimmed.contains("Midi")))
        {
            continue;
        }

        // Skip duplicate struct definitions (we define them in Bevy wrapper)
        if trimmed.starts_with("struct WindowUniform") || trimmed.starts_with("struct TimeUniform")
        {
            skip_lines = true;
            continue;
        }

        // Skip functions that use unavailable resources
        if trimmed.contains("fn Spectrum")
            || trimmed.contains("fn Osc")
            || trimmed.contains("fn Midi")
            || trimmed.contains("fn Mouse")
        {
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
    let common_wgsl = include_str!("../../shaders/common.wgsl");

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
    let mut file = fs::File::create(output_path)?;
    file.write_all(combined_shader.as_bytes())?;

    log::info!(
        "Dynamic shader file generated successfully at {}",
        output_path
    );
    Ok(())
}

// Generate shader using ShaderPreprocessor
#[allow(dead_code)]
fn generate_shader_with_preprocessor(
    config: &crate::ShekereConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    log::info!("Generating shader with ShaderPreprocessor");

    // Create shader preprocessor
    let preprocessor = ShaderPreprocessor::new(&config.config_dir);

    // Get fragment shader path from config
    let fragment_path = config.config_dir.join(&config.config.pipeline[0].file);

    // Process the shader with embedded definitions
    let shader_source = preprocessor
        .process_file_with_embedded_defs_and_multipass(&fragment_path, false)
        .map_err(|e| format!("Failed to process shader: {:?}", e))?;

    log::info!(
        "Shader generated successfully with {} characters",
        shader_source.len()
    );
    Ok(shader_source)
}

// Calculate hash of configuration for change detection
pub(super) fn calculate_config_hash(config: &crate::ShekereConfig) -> u64 {
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
pub fn check_shader_reload(
    config: Res<crate::ShekereConfig>,
    shader_state: Option<ResMut<DynamicShaderState>>,
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

// Generate shader source for a specific pass in multi-pass rendering
pub(super) fn generate_shader_for_pass(
    config: &crate::ShekereConfig,
    pass_index: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    log::info!(
        "Generating shader for pass {} with modular WGSL inclusion",
        pass_index
    );

    // Validate pass index
    if pass_index >= config.config.pipeline.len() {
        return Err(format!(
            "Invalid pass index {}, only {} passes configured",
            pass_index,
            config.config.pipeline.len()
        )
        .into());
    }

    let shader_config = &config.config.pipeline[pass_index];

    // Read the fragment shader file for this pass
    let fragment_path = config.config_dir.join(&shader_config.file);
    let fragment_source = std::fs::read_to_string(&fragment_path).map_err(|e| {
        format!(
            "Failed to read fragment shader {}: {:?}",
            shader_config.file, e
        )
    })?;

    // Check which features the shader uses
    let uses_mouse = fragment_source.contains("MouseCoords");
    let uses_osc = fragment_source.contains("Osc");
    let uses_spectrum = fragment_source.contains("Spectrum");
    let uses_midi = fragment_source.contains("Midi");
    let uses_texture =
        fragment_source.contains("SamplePreviousPass") || fragment_source.contains("previous_pass");

    // For multi-pass: passes after the first always have access to previous_pass texture
    // For persistent: the single pass always has access to previous_pass texture (for trail effects)
    let is_multipass = config.config.pipeline.len() > 1;
    let is_persistent =
        config.config.pipeline.len() == 1 && shader_config.persistent.unwrap_or(false);
    let enable_texture_sampling = (is_multipass && pass_index > 0) || is_persistent;

    log::info!(
        "Pass {}: uses_texture={}, enable_texture_sampling={}, is_multipass={}, is_persistent={}",
        pass_index,
        uses_texture,
        enable_texture_sampling,
        is_multipass,
        is_persistent
    );

    // Start with Bevy import
    let mut shader_parts = vec![
        "#import bevy_sprite::mesh2d_vertex_output::VertexOutput".to_string(),
        "".to_string(),
    ];

    // Always include core definitions
    let core_wgsl = include_str!("../../shaders/core.wgsl");
    shader_parts.push("// === CORE DEFINITIONS ===".to_string());
    shader_parts.push(core_wgsl.to_string());
    shader_parts.push("".to_string());

    // Add conditional features only if used
    if uses_mouse {
        let mouse_wgsl = include_str!("../../shaders/mouse.wgsl");
        shader_parts.push("// === MOUSE DEFINITIONS ===".to_string());
        shader_parts.push(mouse_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including mouse input module", pass_index);
    }

    if uses_spectrum {
        let spectrum_wgsl = include_str!("../../shaders/spectrum.wgsl");
        shader_parts.push("// === SPECTRUM DEFINITIONS ===".to_string());
        shader_parts.push(spectrum_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including spectrum analysis module", pass_index);
    }

    if uses_osc {
        let osc_wgsl = include_str!("../../shaders/osc.wgsl");
        shader_parts.push("// === OSC DEFINITIONS ===".to_string());
        shader_parts.push(osc_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including OSC input module", pass_index);
    }

    if uses_midi {
        let midi_wgsl = include_str!("../../shaders/midi.wgsl");
        shader_parts.push("// === MIDI DEFINITIONS ===".to_string());
        shader_parts.push(midi_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including MIDI input module", pass_index);
    }

    // Include texture module if this pass needs to sample from previous pass
    if enable_texture_sampling || uses_texture {
        let texture_wgsl = include_str!("../../shaders/texture.wgsl");
        shader_parts.push("// === TEXTURE DEFINITIONS ===".to_string());
        shader_parts.push(texture_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including multi-pass texture module", pass_index);
    }

    // Add user fragment shader
    shader_parts.push(format!(
        "// === USER FRAGMENT SHADER (Pass {}) ===",
        pass_index
    ));

    // Fix coordinate usage for Bevy
    // - Use mesh.uv instead of mesh.position.xy
    // - Replace tex_coords with uv (Bevy's VertexOutput uses 'uv' field)
    let processed_shader = fragment_source
        .replace("in.position.xy", "(in.uv * Window.resolution)")
        .replace("mesh.position.xy", "(mesh.uv * Window.resolution)")
        .replace("in.tex_coords", "in.uv")
        .replace(".tex_coords", ".uv");

    shader_parts.push(processed_shader);

    let final_shader = shader_parts.join("\n");

    log::info!(
        "Generated shader for pass {} with {} characters",
        pass_index,
        final_shader.len()
    );

    // DEBUG: Write shader to file for inspection
    let debug_path = format!("/tmp/bevy_shader_pass_{}.wgsl", pass_index);
    if let Err(e) = std::fs::write(&debug_path, &final_shader) {
        log::warn!("Failed to write debug shader: {}", e);
    } else {
        log::info!("Debug shader written to {}", debug_path);
    }

    Ok(final_shader)
}

// Generate clean shader source with modular WGSL inclusion
pub(super) fn generate_clean_shader_source(
    config: &crate::ShekereConfig,
) -> Result<String, Box<dyn std::error::Error>> {
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
    let uses_texture =
        fragment_source.contains("SamplePreviousPass") || fragment_source.contains("previous_pass");

    // Start with Bevy import
    let mut shader_parts = vec![
        "#import bevy_sprite::mesh2d_vertex_output::VertexOutput".to_string(),
        "".to_string(),
    ];

    // Always include core definitions
    let core_wgsl = include_str!("../../shaders/core.wgsl");
    shader_parts.push("// === CORE DEFINITIONS ===".to_string());
    shader_parts.push(core_wgsl.to_string());
    shader_parts.push("".to_string());

    // Add conditional features only if used
    if uses_mouse {
        let mouse_wgsl = include_str!("../../shaders/mouse.wgsl");
        shader_parts.push("// === MOUSE DEFINITIONS ===".to_string());
        shader_parts.push(mouse_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including mouse input module");
    }

    if uses_spectrum {
        let spectrum_wgsl = include_str!("../../shaders/spectrum.wgsl");
        shader_parts.push("// === SPECTRUM DEFINITIONS ===".to_string());
        shader_parts.push(spectrum_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including spectrum analysis module");
    }

    if uses_osc {
        let osc_wgsl = include_str!("../../shaders/osc.wgsl");
        shader_parts.push("// === OSC DEFINITIONS ===".to_string());
        shader_parts.push(osc_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including OSC input module");
    }

    if uses_midi {
        let midi_wgsl = include_str!("../../shaders/midi.wgsl");
        shader_parts.push("// === MIDI DEFINITIONS ===".to_string());
        shader_parts.push(midi_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including MIDI input module");
    }

    if uses_texture {
        let texture_wgsl = include_str!("../../shaders/texture.wgsl");
        shader_parts.push("// === TEXTURE DEFINITIONS ===".to_string());
        shader_parts.push(texture_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Including multi-pass texture module");
    }

    // Add user fragment shader
    shader_parts.push("// === USER FRAGMENT SHADER ===".to_string());

    // Replace function name and fix coordinate usage
    // In Bevy Material2d, mesh.position is fragment coordinates relative to the mesh,
    // not window coordinates. We need to use mesh.uv * Window.resolution instead.
    let processed_shader = fragment_source
        .replace("fn fs_main(", "fn fragment(")
        .replace("in.position.xy", "(in.uv * Window.resolution)")
        .replace("mesh.position.xy", "(mesh.uv * Window.resolution)")
        .replace("in.tex_coords", "in.uv")
        .replace(".tex_coords", ".uv");

    shader_parts.push(processed_shader);

    let final_shader = shader_parts.join("\n");

    log::info!(
        "Generated minimal shader with {} characters",
        final_shader.len()
    );

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
