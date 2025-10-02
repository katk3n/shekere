//! Shader generation and hot-reload functionality

use super::materials::DYNAMIC_SHADER_HANDLE;
use super::state::DynamicShaderState;
use bevy::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

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

// Validate shader source to catch obvious syntax errors before compilation
fn validate_shader_source(source: &str) -> bool {
    // Basic validation checks first (fast path)

    // 1. Check for minimum required content
    if source.trim().is_empty() {
        log::error!("Shader validation failed: Empty shader source");
        return false;
    }

    // 2. Check for fragment function
    let has_fragment_fn = source.contains("fn fragment(") || source.contains("fn fs_main(");
    if !has_fragment_fn {
        log::error!("Shader validation failed: No fragment function found");
        return false;
    }

    // 3. Check for balanced braces (basic syntax check)
    let open_braces = source.matches('{').count();
    let close_braces = source.matches('}').count();
    if open_braces != close_braces {
        log::error!(
            "Shader validation failed: Unbalanced braces (open: {}, close: {})",
            open_braces,
            close_braces
        );
        return false;
    }

    // 4. Check for required Bevy imports for Material2d
    if !source.contains("VertexOutput") {
        log::error!("Shader validation failed: Missing VertexOutput import");
        return false;
    }

    // 5. Use naga to validate WGSL compilation, but remove #import directives first
    // Bevy's #import is a preprocessor directive that naga doesn't understand
    let source_for_naga = remove_bevy_imports(source);

    match naga::front::wgsl::parse_str(&source_for_naga) {
        Ok(_module) => {
            log::info!("✅ Shader WGSL compilation validation passed");
            true
        }
        Err(parse_error) => {
            log::error!(
                "❌ Shader WGSL compilation validation failed:\n{}",
                parse_error
            );

            // Extract error details for better debugging
            let error_string = format!("{}", parse_error);
            if error_string.contains("expected") {
                log::error!("   Hint: Check for missing or incorrect syntax");
            }
            if error_string.contains("unknown") || error_string.contains("no definition in scope") {
                log::error!("   Hint: Check for typos in function or variable names");
            }

            false
        }
    }
}

// Prepare shader for naga validation by removing #import and adding stubs
// Naga doesn't support Bevy's #import directives, but Bevy preprocesses these before compilation
fn remove_bevy_imports(source: &str) -> String {
    // Remove #import lines
    let source_without_imports: String = source
        .lines()
        .filter(|line| !line.trim().starts_with("#import"))
        .collect::<Vec<_>>()
        .join("\n");

    // Add VertexOutput stub if it's being used but not defined
    // This allows naga to validate the shader logic without requiring the actual Bevy import
    if source.contains("VertexOutput") && !source_without_imports.contains("struct VertexOutput") {
        let vertex_output_stub = r#"
// Stub definition for naga validation (replaced by Bevy's #import at runtime)
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}
"#;
        format!("{}\n{}", vertex_output_stub, source_without_imports)
    } else {
        source_without_imports
    }
}

// System to check for shader reload
pub fn check_shader_reload(
    config: Res<crate::ShekereConfig>,
    shader_state: Option<ResMut<DynamicShaderState>>,
    hot_reloader: Option<ResMut<super::state::HotReloaderResource>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    let Some(mut state) = shader_state else {
        return;
    };

    // Check for file changes if hot reload is enabled
    let file_changed = if let Some(hr) = hot_reloader {
        if let Some(ref reloader) = hr.reloader {
            reloader.check_for_changes()
        } else {
            false
        }
    } else {
        false
    };

    let current_hash = calculate_config_hash(&config);
    let config_changed = current_hash != state.last_config_hash;

    // Reload if either file or config changed
    if file_changed || config_changed {
        if file_changed {
            log::info!("🔥 Shader file changed, reloading...");
        } else {
            log::info!("Configuration changed, reloading shader dynamically");
        }

        // Generate new clean shader source
        match generate_clean_shader_source(&config) {
            Ok(new_shader_source) => {
                // Validate shader source before attempting to compile
                if validate_shader_source(&new_shader_source) {
                    // Update the existing shader asset directly
                    let new_shader = Shader::from_wgsl(new_shader_source, "dynamic_shader.wgsl");

                    // Only insert if the existing shader exists (prevents overwriting with broken shader)
                    if shaders.get(&DYNAMIC_SHADER_HANDLE).is_some() {
                        shaders.insert(&DYNAMIC_SHADER_HANDLE, new_shader);
                        log::info!("✅ Shader reloaded successfully");
                    } else {
                        log::warn!("⚠️ Original shader not found, skipping reload");
                    }

                    // Update state only on success
                    state.last_config_hash = current_hash;
                } else {
                    log::error!("❌ Shader validation failed. Keeping existing shader.");
                }
            }
            Err(e) => {
                log::error!(
                    "❌ Shader generation failed: {}. Keeping existing shader.",
                    e
                );
                // Existing pipeline is maintained (graceful degradation)
            }
        }
    }
}

// System to check for multi-pass shader reload
pub fn check_multipass_shader_reload(
    config: Res<crate::ShekereConfig>,
    mut multipass_state: Option<ResMut<super::state::MultiPassState>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    let Some(ref mut state) = multipass_state else {
        return;
    };

    // Check for file changes if hot reload is enabled
    let file_changed = if let Some(ref reloader) = state.hot_reloader {
        reloader.check_for_changes()
    } else {
        return; // Hot reload not enabled
    };

    if file_changed {
        log::info!("🔥 Multi-pass shader file(s) changed, reloading...");

        // Regenerate all shaders in the pipeline
        // Note: Currently we regenerate all passes when any file changes
        // This ensures consistency across the pipeline
        let mut reload_success = true;
        let mut new_shaders = Vec::new();

        // First, try to generate all shaders
        for pass_index in 0..state.pass_count {
            match generate_shader_for_pass(&config, pass_index) {
                Ok(new_shader_source) => {
                    // Validate before storing
                    if validate_shader_source(&new_shader_source) {
                        new_shaders.push(Some(new_shader_source));
                        log::info!("✅ Shader for pass {} validated successfully", pass_index);
                    } else {
                        log::error!(
                            "❌ Shader validation failed for pass {}. Keeping existing shader.",
                            pass_index
                        );
                        new_shaders.push(None);
                        reload_success = false;
                    }
                }
                Err(e) => {
                    log::error!(
                        "❌ Shader generation failed for pass {}: {}. Keeping existing shader.",
                        pass_index,
                        e
                    );
                    new_shaders.push(None);
                    reload_success = false;
                }
            }
        }

        // Only update shaders if ALL passes succeeded
        if reload_success {
            for (pass_index, shader_source_opt) in new_shaders.iter().enumerate() {
                if let Some(shader_source) = shader_source_opt {
                    let shader_name = format!("dynamic_shader_pass_{}.wgsl", pass_index);
                    let new_shader = Shader::from_wgsl(shader_source.clone(), shader_name);

                    // Update the corresponding shader handle
                    shaders.insert(&state.pass_shader_handles[pass_index], new_shader);
                }
            }
            log::info!("✅ All multi-pass shaders reloaded successfully");
        } else {
            log::error!(
                "❌ Some shaders failed validation. Keeping ALL existing shaders to maintain consistency."
            );
        }
    }
}

// System to check for persistent/ping-pong shader reload
pub fn check_persistent_shader_reload(
    config: Res<crate::ShekereConfig>,
    mut persistent_state: Option<ResMut<super::state::PersistentPassState>>,
    mut shaders: ResMut<Assets<Shader>>,
) {
    let Some(ref mut state) = persistent_state else {
        return;
    };

    // Check for file changes if hot reload is enabled
    let file_changed = if let Some(ref reloader) = state.hot_reloader {
        reloader.check_for_changes()
    } else {
        return; // Hot reload not enabled
    };

    if file_changed {
        log::info!("🔥 Persistent shader file changed, reloading...");

        // Generate new shader source
        match generate_clean_shader_source(&config) {
            Ok(new_shader_source) => {
                // Validate shader source before attempting to compile
                if validate_shader_source(&new_shader_source) {
                    // Update the shader asset directly
                    let new_shader = Shader::from_wgsl(new_shader_source, "persistent_shader.wgsl");

                    // Only insert if the existing shader exists
                    if shaders
                        .get(&super::materials::DYNAMIC_SHADER_HANDLE)
                        .is_some()
                    {
                        shaders.insert(&super::materials::DYNAMIC_SHADER_HANDLE, new_shader);
                        log::info!("✅ Persistent shader reloaded successfully");
                    } else {
                        log::warn!("⚠️ Original persistent shader not found, skipping reload");
                    }
                } else {
                    log::error!("❌ Persistent shader validation failed. Keeping existing shader.");
                }
            }
            Err(e) => {
                log::error!(
                    "❌ Persistent shader generation failed: {}. Keeping existing shader.",
                    e
                );
                // Existing pipeline is maintained (graceful degradation)
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
