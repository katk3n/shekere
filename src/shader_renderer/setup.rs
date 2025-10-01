//! Rendering setup and update systems

use super::generation::{
    calculate_config_hash, generate_clean_shader_source, generate_shader_for_pass,
};
use super::materials::*;
use super::state::{DynamicShaderState, MultiPassState, PersistentPassState, RenderPassMarker};
use crate::inputs::midi::MidiShaderData;
use crate::inputs::mouse::MouseShaderData;
use crate::inputs::osc::OscShaderData;
use crate::inputs::spectrum::SpectrumShaderData;
use bevy::prelude::*;
use bevy::render::camera::{ClearColorConfig, RenderTarget};
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::storage::ShaderStorageBuffer;
use bevy::render::view::RenderLayers;

pub(super) fn setup_dynamic_shader_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ShekereShaderMaterial>>,
    materials_pass0: ResMut<Assets<ShekereShaderMaterialPass0>>,
    materials_pass1: ResMut<Assets<ShekereShaderMaterialPass1>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut images: ResMut<Assets<Image>>,
    config: Res<crate::ShekereConfig>,
    windows: Query<&Window>,
) {
    log::info!("Setting up dynamic WGSL shader rendering with Assets<Shader>");

    let Ok(window) = windows.single() else {
        log::error!("Could not get window for shader setup");
        return;
    };

    log::info!("Window found: {}x{}", window.width(), window.height());

    let pass_count = config.config.pipeline.len();
    let is_multipass = pass_count > 1;
    let is_persistent = pass_count == 1 && config.config.pipeline[0].persistent.unwrap_or(false);
    let is_ping_pong = pass_count == 1 && config.config.pipeline[0].ping_pong.unwrap_or(false);

    log::info!(
        "Rendering mode: {} ({} passes)",
        if is_multipass {
            "Multi-pass"
        } else if is_persistent {
            "Persistent (trail effect)"
        } else if is_ping_pong {
            "Ping-pong (feedback effect)"
        } else {
            "Single-pass"
        },
        pass_count
    );

    // Calculate config hash for change detection
    let config_hash = calculate_config_hash(&config);
    log::info!("Calculated config hash: {}", config_hash);

    // Initialize dynamic shader state
    commands.insert_resource(DynamicShaderState {
        last_config_hash: config_hash,
    });

    // Create common storage buffers used by all passes
    let (mouse_buffer_handle, midi_buffer_handle, spectrum_buffer_handle, osc_buffer_handle) =
        create_storage_buffers(&mut storage_buffers);

    // Store buffer handles as a resource for InputManagers to access
    commands.insert_resource(super::state::InputBufferHandles {
        mouse_buffer: mouse_buffer_handle.clone(),
        midi_buffer: midi_buffer_handle.clone(),
        spectrum_buffer: spectrum_buffer_handle.clone(),
        osc_buffer: osc_buffer_handle.clone(),
    });

    if is_multipass {
        setup_multipass_rendering(
            &mut commands,
            &mut meshes,
            &mut materials,
            materials_pass0,
            materials_pass1,
            &mut shaders,
            &mut images,
            &config,
            window,
            mouse_buffer_handle.clone(),
            midi_buffer_handle.clone(),
            spectrum_buffer_handle.clone(),
            osc_buffer_handle.clone(),
        );
    } else if is_persistent || is_ping_pong {
        setup_persistent_rendering(
            &mut commands,
            &mut meshes,
            materials_pass0,
            materials_pass1,
            &mut materials, // Add single-pass material
            &mut shaders,
            &mut images,
            &config,
            window,
            mouse_buffer_handle.clone(),
            midi_buffer_handle.clone(),
            spectrum_buffer_handle.clone(),
            osc_buffer_handle.clone(),
        );
    } else {
        setup_singlepass_rendering(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut shaders,
            &config,
            window,
            mouse_buffer_handle,
            midi_buffer_handle,
            spectrum_buffer_handle,
            osc_buffer_handle,
        );
    }

    log::info!("=== Dynamic WGSL shader rendering setup completed successfully ===");
}

// Create common storage buffers used by all rendering passes
fn create_storage_buffers(
    storage_buffers: &mut ResMut<Assets<ShaderStorageBuffer>>,
) -> (
    Handle<ShaderStorageBuffer>,
    Handle<ShaderStorageBuffer>,
    Handle<ShaderStorageBuffer>,
    Handle<ShaderStorageBuffer>,
) {
    // Create empty storage buffers - they will be populated by input systems
    // Each buffer holds 512 frames of history data

    // Mouse: 512 frames × (2 floats + 2 padding) = 8KB
    let mouse_size = std::mem::size_of::<MouseShaderData>() * 512;
    let mouse_buffer = ShaderStorageBuffer {
        data: Some(vec![0u8; mouse_size]),
        ..Default::default()
    };
    let mouse_buffer_handle = storage_buffers.add(mouse_buffer);

    // MIDI: 512 frames × (32 vec4s × 3 arrays) = 768KB
    let midi_size = std::mem::size_of::<MidiShaderData>() * 512;
    let midi_buffer = ShaderStorageBuffer {
        data: Some(vec![0u8; midi_size]),
        ..Default::default()
    };
    let midi_buffer_handle = storage_buffers.add(midi_buffer);

    // Spectrum: 512 frames × (256 vec4s × 2 arrays + metadata) = large
    let spectrum_size = std::mem::size_of::<SpectrumShaderData>() * 512;
    let spectrum_buffer = ShaderStorageBuffer {
        data: Some(vec![0u8; spectrum_size]),
        ..Default::default()
    };
    let spectrum_buffer_handle = storage_buffers.add(spectrum_buffer);

    // OSC: 512 frames × (4 vec4s × 4 arrays) = 128KB
    let osc_size = std::mem::size_of::<OscShaderData>() * 512;
    let osc_buffer = ShaderStorageBuffer {
        data: Some(vec![0u8; osc_size]),
        ..Default::default()
    };
    let osc_buffer_handle = storage_buffers.add(osc_buffer);

    (
        mouse_buffer_handle,
        midi_buffer_handle,
        spectrum_buffer_handle,
        osc_buffer_handle,
    )
}

// Setup single-pass rendering (existing behavior)
#[allow(clippy::too_many_arguments)]
fn setup_singlepass_rendering(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ShekereShaderMaterial>>,
    shaders: &mut ResMut<Assets<Shader>>,
    config: &crate::ShekereConfig,
    window: &Window,
    mouse_buffer_handle: Handle<ShaderStorageBuffer>,
    midi_buffer_handle: Handle<ShaderStorageBuffer>,
    spectrum_buffer_handle: Handle<ShaderStorageBuffer>,
    osc_buffer_handle: Handle<ShaderStorageBuffer>,
) {
    log::info!("Setting up single-pass rendering");

    // Generate shader source
    let shader_source = match generate_clean_shader_source(config) {
        Ok(source) => {
            log::info!(
                "Successfully generated shader source ({} chars)",
                source.len()
            );
            source
        }
        Err(e) => {
            log::error!("Failed to generate shader source: {}", e);
            return;
        }
    };

    // Create shader asset
    let shader = Shader::from_wgsl(shader_source, "dynamic_shader.wgsl");
    shaders.insert(&DYNAMIC_SHADER_HANDLE, shader);

    log::info!(
        "Created dynamic shader with handle: {:?}",
        DYNAMIC_SHADER_HANDLE
    );

    // Create fullscreen quad mesh
    let mesh = create_fullscreen_quad_mesh();
    let mesh_handle = meshes.add(mesh);

    // Create material
    let material = materials.add(ShekereShaderMaterial {
        resolution: Vec2::new(window.width(), window.height()),
        duration: 0.0,
        mouse_history: mouse_buffer_handle,
        spectrum_history: spectrum_buffer_handle,
        osc_history: osc_buffer_handle,
        midi_history: midi_buffer_handle,
        previous_pass_texture: None,
    });

    // Spawn the fullscreen quad
    commands.spawn((
        Mesh2d(mesh_handle),
        MeshMaterial2d(material),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(
            window.width(),
            window.height(),
            1.0,
        )),
        FullscreenQuad,
    ));

    log::info!("Spawned fullscreen quad with single-pass material");

    // Add standard 2D camera
    commands.spawn(Camera2d);

    log::info!("Spawned standard 2D camera");

    // Initialize hot reloader if enabled
    let hot_reloader = if config
        .config
        .hot_reload
        .as_ref()
        .map_or(false, |hr| hr.enabled)
    {
        let shader_path = config.config_dir.join(&config.config.pipeline[0].file);
        match crate::hot_reload::HotReloader::new(&shader_path) {
            Ok(reloader) => {
                log::info!("✅ Hot reload enabled for: {:?}", shader_path);
                Some(reloader)
            }
            Err(e) => {
                log::warn!("❌ Failed to initialize hot reload: {}", e);
                None
            }
        }
    } else {
        log::info!("Hot reload disabled in configuration");
        None
    };

    commands.insert_resource(super::state::HotReloaderResource {
        reloader: hot_reloader,
        shader_paths: vec![config.config_dir.join(&config.config.pipeline[0].file)],
    });
}

// Setup multi-pass rendering with intermediate textures
#[allow(clippy::too_many_arguments)]
fn setup_multipass_rendering(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<ShekereShaderMaterial>>,
    mut materials_pass0: ResMut<Assets<ShekereShaderMaterialPass0>>,
    mut materials_pass1: ResMut<Assets<ShekereShaderMaterialPass1>>,
    shaders: &mut ResMut<Assets<Shader>>,
    images: &mut ResMut<Assets<Image>>,
    config: &crate::ShekereConfig,
    window: &Window,
    mouse_buffer_handle: Handle<ShaderStorageBuffer>,
    midi_buffer_handle: Handle<ShaderStorageBuffer>,
    spectrum_buffer_handle: Handle<ShaderStorageBuffer>,
    osc_buffer_handle: Handle<ShaderStorageBuffer>,
) {
    log::info!(
        "Setting up multi-pass rendering with {} passes",
        config.config.pipeline.len()
    );

    let pass_count = config.config.pipeline.len();

    // Currently only support 2-pass rendering
    if pass_count != 2 {
        log::error!(
            "Currently only 2-pass rendering is supported, got {}",
            pass_count
        );
        return;
    }

    // Create intermediate texture for pass 0 output
    let intermediate_texture =
        create_intermediate_render_texture(window.width() as u32, window.height() as u32);
    let intermediate_texture_handle = images.add(intermediate_texture);
    log::info!("Created intermediate texture for pass 0");

    // Generate shader for pass 0
    let shader_source_pass0 = match generate_shader_for_pass(config, 0) {
        Ok(source) => {
            log::info!(
                "Successfully generated shader for pass 0 ({} chars)",
                source.len()
            );
            source
        }
        Err(e) => {
            log::error!("Failed to generate shader for pass 0: {}", e);
            return;
        }
    };

    let shader_pass0 = Shader::from_wgsl(shader_source_pass0, "dynamic_shader_pass_0.wgsl");
    shaders.insert(&PASS_0_SHADER_HANDLE, shader_pass0);
    log::info!("Created shader for pass 0");

    // Generate shader for pass 1
    let shader_source_pass1 = match generate_shader_for_pass(config, 1) {
        Ok(source) => {
            log::info!(
                "Successfully generated shader for pass 1 ({} chars)",
                source.len()
            );
            source
        }
        Err(e) => {
            log::error!("Failed to generate shader for pass 1: {}", e);
            return;
        }
    };

    let shader_pass1 = Shader::from_wgsl(shader_source_pass1, "dynamic_shader_pass_1.wgsl");
    shaders.insert(&PASS_1_SHADER_HANDLE, shader_pass1);
    log::info!("Created shader for pass 1");

    // Create fullscreen quad mesh
    let mesh = create_fullscreen_quad_mesh();
    let mesh_handle = meshes.add(mesh);

    // Create material for pass 0 (no previous texture)
    let material_pass0 = materials_pass0.add(ShekereShaderMaterialPass0 {
        resolution: Vec2::new(window.width(), window.height()),
        duration: 0.0,
        mouse_history: mouse_buffer_handle.clone(),
        spectrum_history: spectrum_buffer_handle.clone(),
        osc_history: osc_buffer_handle.clone(),
        midi_history: midi_buffer_handle.clone(),
        previous_pass_texture: None,
    });

    // Create material for pass 1 (uses pass 0 output)
    let material_pass1 = materials_pass1.add(ShekereShaderMaterialPass1 {
        resolution: Vec2::new(window.width(), window.height()),
        duration: 0.0,
        mouse_history: mouse_buffer_handle,
        spectrum_history: spectrum_buffer_handle,
        osc_history: osc_buffer_handle,
        midi_history: midi_buffer_handle,
        previous_pass_texture: Some(intermediate_texture_handle.clone()),
    });

    // ========================================
    // Multi-pass rendering using 2 cameras with RenderLayers
    // ========================================

    // Spawn entity for pass 0 (scene shader, renders to intermediate texture)
    let entity_pass0 = commands
        .spawn((
            Mesh2d(mesh_handle.clone()),
            MeshMaterial2d(material_pass0),
            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(
                window.width(),
                window.height(),
                1.0,
            )),
            FullscreenQuad,
            RenderPassMarker {
                pass_index: 0,
                is_final_pass: false,
            },
            RenderLayers::layer(1), // Only visible to Pass 0 camera
        ))
        .id();

    // Spawn entity for pass 1 (blur shader, renders to screen)
    let entity_pass1 = commands
        .spawn((
            Mesh2d(mesh_handle),
            MeshMaterial2d(material_pass1),
            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(
                window.width(),
                window.height(),
                1.0,
            )),
            FullscreenQuad,
            RenderPassMarker {
                pass_index: 1,
                is_final_pass: true,
            },
            RenderLayers::layer(2), // Only visible to Pass 1 camera
        ))
        .id();

    log::info!("Spawned entities for pass 0 (layer 1) and pass 1 (layer 2)");

    // Camera 0: Renders pass 0 to intermediate texture
    commands.spawn((
        Camera2d,
        Camera {
            order: 0,
            target: RenderTarget::Image(intermediate_texture_handle.clone().into()),
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            ..default()
        },
        RenderLayers::layer(1), // Only sees layer 1 (pass 0 entity)
    ));

    // Camera 1: Renders pass 1 to screen
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            ..default() // Renders to screen
        },
        RenderLayers::layer(2), // Only sees layer 2 (pass 1 entity)
    ));

    log::info!("Created 2 cameras: Pass 0 (renders to texture) -> Pass 1 (renders to screen)");

    // Initialize hot reloader for multi-pass if enabled
    let shader_paths: Vec<std::path::PathBuf> = config
        .config
        .pipeline
        .iter()
        .map(|p| config.config_dir.join(&p.file))
        .collect();

    let hot_reloader = if config
        .config
        .hot_reload
        .as_ref()
        .map_or(false, |hr| hr.enabled)
    {
        match crate::hot_reload::HotReloader::new_multi_file(shader_paths.clone()) {
            Ok(reloader) => {
                log::info!(
                    "✅ Hot reload enabled for {} shader files",
                    shader_paths.len()
                );
                Some(reloader)
            }
            Err(e) => {
                log::warn!("❌ Failed to initialize multi-pass hot reload: {}", e);
                None
            }
        }
    } else {
        log::info!("Hot reload disabled in configuration");
        None
    };

    // Store multi-pass state
    commands.insert_resource(MultiPassState {
        pass_count: 2,
        intermediate_textures: vec![intermediate_texture_handle],
        pass_shader_handles: vec![PASS_0_SHADER_HANDLE, PASS_1_SHADER_HANDLE],
        pass_entities: vec![entity_pass0, entity_pass1],
        hot_reloader,
        shader_paths,
    });

    log::info!("Multi-pass rendering setup completed with 2 cameras and RenderLayers");
}

// Setup persistent texture rendering (trail effects with double-buffering)
// Uses two cameras alternating activation for ping-pong buffering
#[allow(clippy::too_many_arguments)]
fn setup_persistent_rendering(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    _materials_pass0: ResMut<Assets<ShekereShaderMaterialPass0>>,
    mut materials_pass1: ResMut<Assets<ShekereShaderMaterialPass1>>,
    materials: &mut ResMut<Assets<ShekereShaderMaterial>>,
    shaders: &mut ResMut<Assets<Shader>>,
    images: &mut ResMut<Assets<Image>>,
    config: &crate::ShekereConfig,
    window: &Window,
    mouse_buffer_handle: Handle<ShaderStorageBuffer>,
    midi_buffer_handle: Handle<ShaderStorageBuffer>,
    spectrum_buffer_handle: Handle<ShaderStorageBuffer>,
    osc_buffer_handle: Handle<ShaderStorageBuffer>,
) {
    log::info!("Setting up persistent texture rendering with alternating cameras");

    // Create TWO textures for ping-pong buffering
    let texture_a =
        create_intermediate_render_texture(window.width() as u32, window.height() as u32);
    let texture_b =
        create_intermediate_render_texture(window.width() as u32, window.height() as u32);
    let handle_a = images.add(texture_a);
    let handle_b = images.add(texture_b);
    log::info!("Created two textures for ping-pong buffering");

    // Generate trail shader
    let shader_source = match generate_clean_shader_source(config) {
        Ok(source) => {
            log::info!(
                "Successfully generated persistent shader ({} chars)",
                source.len()
            );
            source
        }
        Err(e) => {
            log::error!("Failed to generate persistent shader: {}", e);
            return;
        }
    };

    let shader = Shader::from_wgsl(shader_source, "persistent_shader.wgsl");
    shaders.insert(&DYNAMIC_SHADER_HANDLE, shader);
    log::info!("Created persistent shader");

    // Create simple pass-through shader for display
    let display_shader_source = r#"
#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(2) @binding(0) var<uniform> resolution: vec2<f32>;
@group(2) @binding(1) var<uniform> duration: f32;
@group(2) @binding(2) var<storage, read> mouse_history: array<vec4<f32>>;
@group(2) @binding(3) var<storage, read> spectrum_history: array<vec4<f32>>;
@group(2) @binding(4) var<storage, read> osc_history: array<vec4<f32>>;
@group(2) @binding(5) var<storage, read> midi_history: array<vec4<f32>>;
@group(2) @binding(6) var previous_pass: texture_2d<f32>;
@group(2) @binding(7) var texture_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple pass-through: display the texture
    return textureSample(previous_pass, texture_sampler, in.uv);
}
"#;

    let display_shader =
        Shader::from_wgsl(display_shader_source.to_string(), "persistent_display.wgsl");
    shaders.insert(&PASS_1_SHADER_HANDLE, display_shader);
    log::info!("Created pass-through display shader");

    // Create fullscreen quad mesh
    let mesh = create_fullscreen_quad_mesh();
    let mesh_handle = meshes.add(mesh);

    // Create material for trail rendering (reads from texture_b initially)
    let material_trail = materials.add(ShekereShaderMaterial {
        resolution: Vec2::new(window.width(), window.height()),
        duration: 0.0,
        mouse_history: mouse_buffer_handle.clone(),
        spectrum_history: spectrum_buffer_handle.clone(),
        osc_history: osc_buffer_handle.clone(),
        midi_history: midi_buffer_handle.clone(),
        previous_pass_texture: Some(handle_b.clone()),
    });

    // Create material for display (reads from texture_a initially)
    let material_display = materials_pass1.add(ShekereShaderMaterialPass1 {
        resolution: Vec2::new(window.width(), window.height()),
        duration: 0.0,
        mouse_history: mouse_buffer_handle,
        spectrum_history: spectrum_buffer_handle,
        osc_history: osc_buffer_handle,
        midi_history: midi_buffer_handle,
        previous_pass_texture: Some(handle_a.clone()),
    });

    // Spawn trail entity (visible to cameras A and B)
    let entity_trail = commands
        .spawn((
            Mesh2d(mesh_handle.clone()),
            MeshMaterial2d(material_trail),
            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(
                window.width(),
                window.height(),
                1.0,
            )),
            FullscreenQuad,
            RenderLayers::layer(1), // Visible to render cameras
        ))
        .id();

    // Spawn display entity (visible to display camera only) - uses Pass1 material
    let entity_display = commands
        .spawn((
            Mesh2d(mesh_handle),
            MeshMaterial2d(material_display),
            Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::new(
                window.width(),
                window.height(),
                1.0,
            )),
            FullscreenQuad,
            RenderLayers::layer(2), // Visible to display camera
        ))
        .id();

    log::info!("Spawned trail entity and display entity");

    // Camera A: Renders trail entity to texture_a (initially active)
    let camera_a = commands
        .spawn((
            Camera2d,
            Camera {
                order: 0,
                is_active: true, // Start active
                target: RenderTarget::Image(handle_a.clone().into()),
                clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                ..default()
            },
            RenderLayers::layer(1),
        ))
        .id();

    // Camera B: Renders trail entity to texture_b (initially inactive)
    let camera_b = commands
        .spawn((
            Camera2d,
            Camera {
                order: 0,
                is_active: false, // Start inactive
                target: RenderTarget::Image(handle_b.clone().into()),
                clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                ..default()
            },
            RenderLayers::layer(1),
        ))
        .id();

    // Display Camera: Renders display entity to screen
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            ..default()
        },
        RenderLayers::layer(2),
    ));

    log::info!("Created alternating cameras A & B, and display camera");

    // Initialize hot reloader for persistent if enabled
    let shader_path = config.config_dir.join(&config.config.pipeline[0].file);
    let hot_reloader = if config
        .config
        .hot_reload
        .as_ref()
        .map_or(false, |hr| hr.enabled)
    {
        match crate::hot_reload::HotReloader::new(&shader_path) {
            Ok(reloader) => {
                log::info!(
                    "✅ Hot reload enabled for persistent shader: {:?}",
                    shader_path
                );
                Some(reloader)
            }
            Err(e) => {
                log::warn!("❌ Failed to initialize persistent hot reload: {}", e);
                None
            }
        }
    } else {
        log::info!("Hot reload disabled in configuration");
        None
    };

    // Store state
    commands.insert_resource(PersistentPassState {
        frame_count: 0,
        textures: [handle_a, handle_b],
        entity: entity_trail,
        camera_a,
        camera_b,
        display_entity: entity_display,
        hot_reloader,
        shader_path,
    });

    log::info!("Persistent rendering setup completed with alternating camera architecture");
}

// Create fullscreen quad mesh
fn create_fullscreen_quad_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    // Fullscreen quad vertices in unit space (will be scaled by Transform)
    let vertices = vec![
        [-0.5, -0.5, 0.0], // bottom left
        [0.5, -0.5, 0.0],  // bottom right
        [0.5, 0.5, 0.0],   // top right
        [-0.5, 0.5, 0.0],  // top left
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

    mesh
}

// Create intermediate render texture
fn create_intermediate_render_texture(width: u32, height: u32) -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_DST;

    image
}

// Update shader uniforms every frame
pub fn update_shader_uniforms(
    time: Res<Time>,
    windows: Query<&Window>,
    mut materials: ResMut<Assets<ShekereShaderMaterial>>,
    quad_query: Query<&MeshMaterial2d<ShekereShaderMaterial>, With<FullscreenQuad>>,
) {
    let Ok(window) = windows.single() else {
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
        log::info!(
            "Updated {} materials - time: {:.2}s",
            updated_count,
            elapsed
        );
    }
}

// Update multi-pass shader uniforms every frame
pub fn update_multipass_uniforms(
    time: Res<Time>,
    windows: Query<&Window>,
    mut materials_pass0: ResMut<Assets<ShekereShaderMaterialPass0>>,
    mut materials_pass1: ResMut<Assets<ShekereShaderMaterialPass1>>,
    query_pass0: Query<&MeshMaterial2d<ShekereShaderMaterialPass0>, With<FullscreenQuad>>,
    query_pass1: Query<&MeshMaterial2d<ShekereShaderMaterialPass1>, With<FullscreenQuad>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let elapsed = time.elapsed_secs();

    // Update pass 0 materials
    for material_handle in query_pass0.iter() {
        if let Some(material) = materials_pass0.get_mut(&material_handle.0) {
            material.resolution = Vec2::new(window.width(), window.height());
            material.duration = elapsed;
        }
    }

    // Update pass 1 materials
    for material_handle in query_pass1.iter() {
        if let Some(material) = materials_pass1.get_mut(&material_handle.0) {
            material.resolution = Vec2::new(window.width(), window.height());
            material.duration = elapsed;
        }
    }
}

// Update persistent texture shader uniforms every frame with double-buffering
// Alternates camera activation and swaps textures for ping-pong effect
#[allow(clippy::too_many_arguments)]
pub fn update_persistent_uniforms(
    time: Res<Time>,
    windows: Query<&Window>,
    mut persistent_state: ResMut<PersistentPassState>,
    mut materials: ResMut<Assets<ShekereShaderMaterial>>,
    mut materials_pass1: ResMut<Assets<ShekereShaderMaterialPass1>>,
    mut cameras: Query<&mut Camera>,
    query_trail: Query<&MeshMaterial2d<ShekereShaderMaterial>>,
    query_display: Query<&MeshMaterial2d<ShekereShaderMaterialPass1>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let elapsed = time.elapsed_secs();

    // Update frame count
    persistent_state.frame_count += 1;
    let frame = persistent_state.frame_count;

    // Determine which camera should be active this frame
    // Even frames: Camera A active (writes to texture_a, reads from texture_b)
    // Odd frames: Camera B active (writes to texture_b, reads from texture_a)
    let use_camera_a = frame % 2 == 0;

    if frame <= 3 {
        log::info!(
            "Persistent frame {}: using camera {}",
            frame,
            if use_camera_a { "A" } else { "B" }
        );
    }

    // Toggle camera activation
    if let Ok(mut cam_a) = cameras.get_mut(persistent_state.camera_a) {
        cam_a.is_active = use_camera_a;
    }
    if let Ok(mut cam_b) = cameras.get_mut(persistent_state.camera_b) {
        cam_b.is_active = !use_camera_a;
    }

    // Determine texture indices
    let (write_index, read_index) = if use_camera_a {
        (0, 1) // Camera A writes to texture_a (0), reads from texture_b (1)
    } else {
        (1, 0) // Camera B writes to texture_b (1), reads from texture_a (0)
    };

    // Update trail entity material (reads from the texture NOT being written to)
    if let Ok(material_handle) = query_trail.get(persistent_state.entity) {
        if let Some(material) = materials.get_mut(&material_handle.0) {
            material.resolution = Vec2::new(window.width(), window.height());
            material.duration = elapsed;
            material.previous_pass_texture = Some(persistent_state.textures[read_index].clone());

            if frame <= 3 {
                log::info!("Trail entity: reading from texture {}", read_index);
            }
        }
    }

    // Update display entity material (reads from the texture being written to)
    if let Ok(material_handle) = query_display.get(persistent_state.display_entity) {
        if let Some(material) = materials_pass1.get_mut(&material_handle.0) {
            material.resolution = Vec2::new(window.width(), window.height());
            material.duration = elapsed;
            material.previous_pass_texture = Some(persistent_state.textures[write_index].clone());

            if frame <= 3 {
                log::info!("Display entity: reading from texture {}", write_index);
            }
        }
    }
}
