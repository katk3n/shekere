// WGSL shader rendering using Bevy's material system
// This integrates actual WGSL shaders from configuration files

use crate::shader_preprocessor::ShaderPreprocessor;
use bevy::prelude::*;
use bevy::render::camera::{ClearColorConfig, RenderTarget};
use bevy::render::mesh::MeshVertexBufferLayoutRef;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderRef, ShaderType, SpecializedMeshPipelineError,
};
use bevy::render::storage::ShaderStorageBuffer;
use bevy::render::view::RenderLayers;
use bevy::sprite::{Material2d, Material2dKey, Material2dPlugin};
use bytemuck::{Pod, Zeroable};
use ringbuf::{HeapRb, traits::*};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;

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

// Mouse frame data for ring buffer storage
#[derive(Debug, Clone, Copy)]
struct MouseFrameData {
    position: [f32; 2],
}

impl MouseFrameData {
    fn new(x: f32, y: f32) -> Self {
        Self { position: [x, y] }
    }

    fn to_shader_data(&self) -> MouseShaderData {
        MouseShaderData {
            position: self.position,
            _padding: [0.0, 0.0],
        }
    }
}

// Resource to track mouse history locally
#[derive(Resource)]
struct MouseHistoryTracker {
    current_frame: MouseFrameData,
    ring_buffer: HeapRb<MouseFrameData>,
}

impl Default for MouseHistoryTracker {
    fn default() -> Self {
        Self {
            current_frame: MouseFrameData::new(0.0, 0.0),
            ring_buffer: HeapRb::new(512),
        }
    }
}

// MIDI data structures for GPU
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
struct MidiShaderData {
    // note velocities (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    notes: [[f32; 4]; 32],
    // control change values (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    controls: [[f32; 4]; 32],
    // note on attack detection (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    note_on: [[f32; 4]; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
struct MidiHistoryBuffer {
    // 512 frames of MIDI history data
    history_data: [MidiShaderData; 512],
}

// OSC data structures for GPU
use crate::inputs::osc::{OscHistoryData, OscShaderData};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
struct OscHistoryBuffer {
    // 512 frames of OSC history data
    history_data: [OscShaderData; 512],
}

// Resource to track OSC history locally
#[derive(Resource)]
struct OscHistoryTracker {
    history_data: std::sync::Arc<std::sync::Mutex<OscHistoryData>>,
    receiver: Option<async_std::channel::Receiver<rosc::OscPacket>>,
    sound_map: std::collections::HashMap<String, i32>,
}

impl Default for OscHistoryTracker {
    fn default() -> Self {
        Self {
            history_data: std::sync::Arc::new(std::sync::Mutex::new(OscHistoryData::new())),
            receiver: None,
            sound_map: std::collections::HashMap::new(),
        }
    }
}

// Resource to track MIDI history locally
#[derive(Resource)]
struct MidiHistoryTracker {
    current_frame: std::sync::Arc<std::sync::Mutex<MidiFrameData>>,
    ring_buffer: HeapRb<MidiShaderData>,
    _midi_connection: Option<midir::MidiInputConnection<()>>,
}

// Internal MIDI frame data (not GPU-aligned)
struct MidiFrameData {
    notes: [f32; 128],
    controls: [f32; 128],
    note_on: [f32; 128],
}

impl MidiFrameData {
    fn new() -> Self {
        Self {
            notes: [0.0; 128],
            controls: [0.0; 128],
            note_on: [0.0; 128],
        }
    }

    fn clear_note_on(&mut self) {
        self.note_on = [0.0; 128];
    }

    fn to_shader_data(&self) -> MidiShaderData {
        let mut notes = [[0.0f32; 4]; 32];
        let mut controls = [[0.0f32; 4]; 32];
        let mut note_on = [[0.0f32; 4]; 32];

        // Pack arrays into vec4 format
        for i in 0..128 {
            let vec4_index = i / 4;
            let element_index = i % 4;
            notes[vec4_index][element_index] = self.notes[i];
            controls[vec4_index][element_index] = self.controls[i];
            note_on[vec4_index][element_index] = self.note_on[i];
        }

        MidiShaderData {
            notes,
            controls,
            note_on,
        }
    }
}

impl Default for MidiHistoryTracker {
    fn default() -> Self {
        Self {
            current_frame: std::sync::Arc::new(std::sync::Mutex::new(MidiFrameData::new())),
            ring_buffer: HeapRb::new(512),
            _midi_connection: None,
        }
    }
}
// Spectrum data structures for GPU
const SPECTRUM_NUM_SAMPLES: usize = 2048;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, ShaderType)]
struct SpectrumShaderData {
    // frequency values of audio input (packed into vec4s for alignment)
    // 2048 samples / 4 = 512 vec4s
    frequencies: [[f32; 4]; SPECTRUM_NUM_SAMPLES / 4],
    // amplitude values of audio input (packed into vec4s for alignment)
    // 2048 samples / 4 = 512 vec4s
    amplitudes: [[f32; 4]; SPECTRUM_NUM_SAMPLES / 4],
    // the number of data points
    num_points: u32,
    // frequency of the data point with the max amplitude
    max_frequency: f32,
    // max amplitude of audio input
    max_amplitude: f32,
    _padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
struct SpectrumHistoryBuffer {
    // 512 frames of spectrum history data
    history_data: [SpectrumShaderData; 512],
}

// Internal spectrum frame data (updated by audio processing)
#[derive(Debug, Clone, Copy)]
struct SpectrumFrameData {
    frequencies: [[f32; 4]; SPECTRUM_NUM_SAMPLES / 4],
    amplitudes: [[f32; 4]; SPECTRUM_NUM_SAMPLES / 4],
    num_points: u32,
    max_frequency: f32,
    max_amplitude: f32,
}

impl Default for SpectrumFrameData {
    fn default() -> Self {
        Self {
            frequencies: [[0.0; 4]; SPECTRUM_NUM_SAMPLES / 4],
            amplitudes: [[0.0; 4]; SPECTRUM_NUM_SAMPLES / 4],
            num_points: 0,
            max_frequency: 0.0,
            max_amplitude: 0.0,
        }
    }
}

impl SpectrumFrameData {
    fn to_shader_data(&self) -> SpectrumShaderData {
        SpectrumShaderData {
            frequencies: self.frequencies,
            amplitudes: self.amplitudes,
            num_points: self.num_points,
            max_frequency: self.max_frequency,
            max_amplitude: self.max_amplitude,
            _padding: 0,
        }
    }
}

// Wrapper for audio stream components
struct SpectrumAudioStream {
    consumer: ringbuf::wrap::caching::Caching<std::sync::Arc<ringbuf::HeapRb<f32>>, false, true>,
    _stream: cpal::Stream,
}

// cpal::Stream is designed to be thread-safe despite not implementing Send/Sync
// The stream handle can be safely stored and the audio callback runs on a separate thread
unsafe impl Send for SpectrumAudioStream {}
unsafe impl Sync for SpectrumAudioStream {}

// Resource to track spectrum history locally
// Uses Vec for history to avoid HeapRb API complications
#[derive(Resource)]
struct SpectrumHistoryTracker {
    current_frame: SpectrumFrameData,
    history: std::collections::VecDeque<SpectrumFrameData>,
    audio_stream: Option<SpectrumAudioStream>,
    min_frequency: f32,
    max_frequency: f32,
    sampling_rate: u32,
}

impl Default for SpectrumHistoryTracker {
    fn default() -> Self {
        Self {
            current_frame: SpectrumFrameData::default(),
            history: std::collections::VecDeque::with_capacity(512),
            audio_stream: None,
            min_frequency: 27.0,
            max_frequency: 2000.0,
            sampling_rate: 44100,
        }
    }
}

impl SpectrumHistoryTracker {
    // Prepare shader data from current frame and history
    fn prepare_shader_data(&self) -> Vec<SpectrumShaderData> {
        let mut shader_data = Vec::with_capacity(512);

        // Add current frame first (index 0 = most recent)
        shader_data.push(self.current_frame.to_shader_data());

        // Add frames from history (newest to oldest)
        for frame in self.history.iter().rev() {
            shader_data.push(frame.to_shader_data());
            if shader_data.len() >= 512 {
                break;
            }
        }

        // Pad to exactly 512 frames if needed
        while shader_data.len() < 512 {
            shader_data.push(SpectrumFrameData::default().to_shader_data());
        }

        shader_data
    }
}

// Plugin for simple shader rendering
pub struct SimpleShaderRenderPlugin;

impl Plugin for SimpleShaderRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            Material2dPlugin::<ShekerShaderMaterial>::default(),
            Material2dPlugin::<ShekerShaderMaterialPass0>::default(),
            Material2dPlugin::<ShekerShaderMaterialPass1>::default(),
        ))
        .init_resource::<MouseHistoryTracker>()
        .init_resource::<MidiHistoryTracker>()
        .init_resource::<SpectrumHistoryTracker>()
        .init_resource::<OscHistoryTracker>()
        .add_systems(
            Startup,
            (
                setup_dynamic_shader_system,
                setup_midi_system,
                setup_spectrum_system,
                setup_osc_system,
            ),
        )
        .add_systems(
            Update,
            (
                update_shader_uniforms,
                update_multipass_uniforms,
                update_mouse_history,
                update_midi_system,
                update_spectrum_system,
                update_osc_system,
                check_shader_reload,
            ),
        );
    }
}

// Constant handle for our dynamic shader
const DYNAMIC_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x9E4B8A2F1C6D3E7F8A9B4C5D6E7F8A9B);

// Resource to hold dynamic shader state
#[derive(Resource)]
struct DynamicShaderState {
    last_config_hash: u64,
}

// Resource to track multi-pass rendering state
#[derive(Resource)]
struct MultiPassState {
    pass_count: usize,
    intermediate_textures: Vec<Handle<Image>>,
    pass_shader_handles: Vec<Handle<Shader>>,
    pass_entities: Vec<Entity>,
}

// Component to mark render pass entities
#[derive(Component)]
struct RenderPassMarker {
    pass_index: usize,
    is_final_pass: bool,
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
    #[storage(3, read_only)]
    spectrum_history: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)]
    osc_history: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)]
    midi_history: Handle<ShaderStorageBuffer>,
    // Multi-pass texture bindings (optional - only used in multi-pass rendering)
    #[texture(6)]
    #[sampler(7)]
    previous_pass_texture: Option<Handle<Image>>,
}

impl Material2d for ShekerShaderMaterial {
    fn fragment_shader() -> ShaderRef {
        // Always return our fixed dynamic shader handle
        DYNAMIC_SHADER_HANDLE.into()
    }
}
// Pass-specific material types for multi-pass rendering
// Each pass needs its own Material type because Material2d::fragment_shader() is static

const PASS_0_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x9E4B8A2F1C6D3E7F8A9B4C5D6E7F8A9C);
const PASS_1_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x9E4B8A2F1C6D3E7F8A9B4C5D6E7F8A9D);

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct ShekerShaderMaterialPass0 {
    #[uniform(0)]
    resolution: Vec2,
    #[uniform(1)]
    duration: f32,
    #[storage(2, read_only)]
    mouse_history: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)]
    spectrum_history: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)]
    osc_history: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)]
    midi_history: Handle<ShaderStorageBuffer>,
    #[texture(6)]
    #[sampler(7)]
    previous_pass_texture: Option<Handle<Image>>,
}

impl Material2d for ShekerShaderMaterialPass0 {
    fn fragment_shader() -> ShaderRef {
        PASS_0_SHADER_HANDLE.into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set custom entry point name to match user shader
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = "fs_main".into();
        }
        Ok(())
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
struct ShekerShaderMaterialPass1 {
    #[uniform(0)]
    resolution: Vec2,
    #[uniform(1)]
    duration: f32,
    #[storage(2, read_only)]
    mouse_history: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)]
    spectrum_history: Handle<ShaderStorageBuffer>,
    #[storage(4, read_only)]
    osc_history: Handle<ShaderStorageBuffer>,
    #[storage(5, read_only)]
    midi_history: Handle<ShaderStorageBuffer>,
    #[texture(6)]
    #[sampler(7)]
    previous_pass_texture: Option<Handle<Image>>,
}

impl Material2d for ShekerShaderMaterialPass1 {
    fn fragment_shader() -> ShaderRef {
        PASS_1_SHADER_HANDLE.into()
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Set custom entry point name to match user shader
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.entry_point = "fs_main".into();
        }
        Ok(())
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
    mut materials_pass0: ResMut<Assets<ShekerShaderMaterialPass0>>,
    mut materials_pass1: ResMut<Assets<ShekerShaderMaterialPass1>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut images: ResMut<Assets<Image>>,
    config: Res<crate::ShekerConfig>,
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

    log::info!(
        "Rendering mode: {} ({} passes)",
        if is_multipass {
            "Multi-pass"
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
            &window,
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
            &window,
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
    // Initialize mouse history buffer with zeros
    let mouse_history_data = MouseHistoryBuffer {
        history_data: [MouseShaderData {
            position: [0.0, 0.0],
            _padding: [0.0, 0.0],
        }; 512],
    };
    let mouse_buffer_handle = storage_buffers.add(ShaderStorageBuffer::from(mouse_history_data));

    // Initialize MIDI history buffer with zeros
    let midi_history_data = MidiHistoryBuffer {
        history_data: [MidiFrameData::new().to_shader_data(); 512],
    };
    let midi_buffer_handle = storage_buffers.add(ShaderStorageBuffer::from(midi_history_data));

    // Initialize Spectrum history buffer with zeros
    let empty_frame = SpectrumFrameData::default();
    let initial_history_vec = vec![empty_frame.to_shader_data(); 512];
    let initial_bytes = bytemuck::cast_slice(&initial_history_vec).to_vec();
    let spectrum_buffer = ShaderStorageBuffer {
        data: Some(initial_bytes),
        ..Default::default()
    };
    let spectrum_buffer_handle = storage_buffers.add(spectrum_buffer);

    // Initialize OSC history buffer with zeros
    let empty_osc_frame = OscShaderData {
        sounds: [[0; 4]; 4],
        ttls: [[0.0; 4]; 4],
        notes: [[0.0; 4]; 4],
        gains: [[0.0; 4]; 4],
    };
    let osc_history_data = OscHistoryBuffer {
        history_data: [empty_osc_frame; 512],
    };
    let osc_buffer_handle = storage_buffers.add(ShaderStorageBuffer::from(osc_history_data));

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
    materials: &mut ResMut<Assets<ShekerShaderMaterial>>,
    shaders: &mut ResMut<Assets<Shader>>,
    config: &crate::ShekerConfig,
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
    let material = materials.add(ShekerShaderMaterial {
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
}

// Setup multi-pass rendering with intermediate textures
#[allow(clippy::too_many_arguments)]
fn setup_multipass_rendering(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<ShekerShaderMaterial>>,
    mut materials_pass0: ResMut<Assets<ShekerShaderMaterialPass0>>,
    mut materials_pass1: ResMut<Assets<ShekerShaderMaterialPass1>>,
    shaders: &mut ResMut<Assets<Shader>>,
    images: &mut ResMut<Assets<Image>>,
    config: &crate::ShekerConfig,
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
    let material_pass0 = materials_pass0.add(ShekerShaderMaterialPass0 {
        resolution: Vec2::new(window.width(), window.height()),
        duration: 0.0,
        mouse_history: mouse_buffer_handle.clone(),
        spectrum_history: spectrum_buffer_handle.clone(),
        osc_history: osc_buffer_handle.clone(),
        midi_history: midi_buffer_handle.clone(),
        previous_pass_texture: None,
    });

    // Create material for pass 1 (uses pass 0 output)
    let material_pass1 = materials_pass1.add(ShekerShaderMaterialPass1 {
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
        Camera2d::default(),
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
        Camera2d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(Color::srgba(0.0, 0.0, 0.0, 1.0)),
            ..default() // Renders to screen
        },
        RenderLayers::layer(2), // Only sees layer 2 (pass 1 entity)
    ));

    log::info!("Created 2 cameras: Pass 0 (renders to texture) -> Pass 1 (renders to screen)");

    // Store multi-pass state
    commands.insert_resource(MultiPassState {
        pass_count: 2,
        intermediate_textures: vec![intermediate_texture_handle],
        pass_shader_handles: vec![PASS_0_SHADER_HANDLE, PASS_1_SHADER_HANDLE],
        pass_entities: vec![entity_pass0, entity_pass1],
    });

    log::info!("Multi-pass rendering setup completed with 2 cameras and RenderLayers");
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
        log::info!(
            "Updated {} materials - time: {:.2}s",
            updated_count,
            elapsed
        );
    }
}

// Update multi-pass shader uniforms every frame
fn update_multipass_uniforms(
    time: Res<Time>,
    windows: Query<&Window>,
    mut materials_pass0: ResMut<Assets<ShekerShaderMaterialPass0>>,
    mut materials_pass1: ResMut<Assets<ShekerShaderMaterialPass1>>,
    query_pass0: Query<&MeshMaterial2d<ShekerShaderMaterialPass0>, With<FullscreenQuad>>,
    query_pass1: Query<&MeshMaterial2d<ShekerShaderMaterialPass1>, With<FullscreenQuad>>,
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
        mouse_tracker.current_frame.position
    };

    // Update current frame position
    mouse_tracker.current_frame.position = mouse_pos;

    // Push current frame to ring buffer (every frame for smooth animation)
    let current_frame = mouse_tracker.current_frame;
    mouse_tracker.ring_buffer.push_overwrite(current_frame);

    // Prepare shader data from ring buffer
    let mut shader_data_vec = Vec::with_capacity(512);

    // Add current frame first (index 0)
    shader_data_vec.push(current_frame.to_shader_data());

    // Add history frames in reverse order (newest to oldest)
    // Limit to 511 frames to ensure total is exactly 512
    for frame in mouse_tracker.ring_buffer.iter().rev() {
        if shader_data_vec.len() >= 512 {
            break;
        }
        shader_data_vec.push(frame.to_shader_data());
    }

    // Fill remaining slots with zeros if ring buffer not full yet
    while shader_data_vec.len() < 512 {
        shader_data_vec.push(MouseShaderData {
            position: [0.0, 0.0],
            _padding: [0.0, 0.0],
        });
    }

    // Create array from vec
    let mut history_array = [MouseShaderData {
        position: [0.0, 0.0],
        _padding: [0.0, 0.0],
    }; 512];
    history_array.copy_from_slice(&shader_data_vec[..512]);

    // Create GPU buffer data
    let buffer_data = MouseHistoryBuffer {
        history_data: history_array,
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

// Setup MIDI input system
fn setup_midi_system(
    mut midi_tracker: ResMut<MidiHistoryTracker>,
    config: Res<crate::ShekerConfig>,
) {
    // Check if MIDI is enabled in config
    let midi_enabled = config
        .config
        .midi
        .as_ref()
        .map(|m| m.enabled)
        .unwrap_or(false);

    if !midi_enabled {
        log::info!("MIDI input disabled in configuration");
        return;
    }

    log::info!("Setting up MIDI input system");

    // Try to setup MIDI input with shared current_frame
    match setup_midi_connection(std::sync::Arc::clone(&midi_tracker.current_frame)) {
        Some(connection) => {
            midi_tracker._midi_connection = Some(connection);
            log::info!("MIDI input system setup completed successfully");
        }
        None => {
            log::warn!("MIDI input setup failed or no devices available");
        }
    }
}

// Helper function to setup MIDI connection
fn setup_midi_connection(
    midi_state: std::sync::Arc<std::sync::Mutex<MidiFrameData>>,
) -> Option<midir::MidiInputConnection<()>> {
    use midir::MidiInput;

    let midi_in = MidiInput::new("shekere MIDI Input").ok()?;

    // Get available ports
    let in_ports = midi_in.ports();
    if in_ports.is_empty() {
        log::warn!("No MIDI input ports available");
        return None;
    }

    // Use the first available port
    let in_port = &in_ports[0];
    let port_name = midi_in
        .port_name(in_port)
        .unwrap_or_else(|_| "Unknown".to_string());
    log::info!("Connecting to MIDI port: {}", port_name);

    let connection = midi_in.connect(
        in_port,
        "shekere-midi",
        move |_timestamp, message, _| {
            handle_midi_message(&midi_state, message);
        },
        (),
    );

    match connection {
        Ok(conn) => {
            log::info!("MIDI input connected successfully");
            Some(conn)
        }
        Err(e) => {
            log::error!("Failed to connect MIDI input: {}", e);
            None
        }
    }
}

// Handle incoming MIDI messages
fn handle_midi_message(state: &std::sync::Arc<std::sync::Mutex<MidiFrameData>>, message: &[u8]) {
    if message.len() < 2 {
        return;
    }

    let mut current_frame = state.lock().unwrap();

    match message[0] & 0xF0 {
        // Note On (0x90)
        0x90 => {
            if message.len() >= 3 {
                let note = message[1] as usize;
                let velocity = message[2] as f32 / 127.0;
                if note < 128 {
                    // Set sustained note
                    current_frame.notes[note] = velocity;
                    // Set attack detection
                    current_frame.note_on[note] = velocity;
                }
            }
        }
        // Note Off (0x80)
        0x80 => {
            if message.len() >= 3 {
                let note = message[1] as usize;
                if note < 128 {
                    current_frame.notes[note] = 0.0;
                }
            }
        }
        // Control Change (0xB0)
        0xB0 => {
            if message.len() >= 3 {
                let controller = message[1] as usize;
                let value = message[2] as f32 / 127.0;
                if controller < 128 {
                    current_frame.controls[controller] = value;
                }
            }
        }
        _ => {
            // Ignore other message types
        }
    }
}

// Update MIDI history in the materials
fn update_midi_system(
    materials: Res<Assets<ShekerShaderMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut midi_tracker: ResMut<MidiHistoryTracker>,
    quad_query: Query<&MeshMaterial2d<ShekerShaderMaterial>, With<FullscreenQuad>>,
) {
    // Lock and convert current frame to shader data
    let current_shader_data = {
        let current_frame = midi_tracker.current_frame.lock().unwrap();
        current_frame.to_shader_data()
    };

    // Push current frame to ring buffer
    midi_tracker.ring_buffer.push_overwrite(current_shader_data);

    // Clear note_on array for next frame (attack detection)
    {
        let mut current_frame = midi_tracker.current_frame.lock().unwrap();
        current_frame.clear_note_on();
    }

    // Prepare shader data from ring buffer
    let mut shader_data_vec = Vec::with_capacity(512);

    // Add current frame first (index 0)
    shader_data_vec.push(current_shader_data);

    // Add history frames in reverse order (newest to oldest)
    // Limit to 511 frames to ensure total is exactly 512
    for frame in midi_tracker.ring_buffer.iter().rev() {
        if shader_data_vec.len() >= 512 {
            break;
        }
        shader_data_vec.push(*frame);
    }

    // Fill remaining slots with zeros if ring buffer not full yet
    let empty_shader_data = MidiFrameData::new().to_shader_data();
    while shader_data_vec.len() < 512 {
        shader_data_vec.push(empty_shader_data);
    }

    // Create array from vec
    let mut history_array = [empty_shader_data; 512];
    history_array.copy_from_slice(&shader_data_vec[..512]);

    // Create GPU buffer data
    let buffer_data = MidiHistoryBuffer {
        history_data: history_array,
    };

    // Update storage buffers for all materials
    for material_handle in quad_query.iter() {
        if let Some(material) = materials.get(&material_handle.0) {
            if let Some(buffer) = storage_buffers.get_mut(&material.midi_history) {
                buffer.set_data(buffer_data);
            }
        }
    }
}

// Setup Spectrum input system
fn setup_spectrum_system(
    mut spectrum_tracker: ResMut<SpectrumHistoryTracker>,
    config: Res<crate::ShekerConfig>,
) {
    // Check if spectrum is enabled in config
    let spectrum_config = match &config.config.spectrum {
        Some(cfg) => cfg,
        None => {
            log::info!("Spectrum analysis disabled in configuration");
            return;
        }
    };

    log::info!("Setting up Spectrum analysis system");

    // Set spectrum configuration
    spectrum_tracker.min_frequency = spectrum_config.min_frequency;
    spectrum_tracker.max_frequency = spectrum_config.max_frequency;
    spectrum_tracker.sampling_rate = spectrum_config.sampling_rate;

    // Try to setup audio stream
    match setup_audio_stream() {
        Ok((stream, consumer)) => {
            spectrum_tracker.audio_stream = Some(SpectrumAudioStream {
                consumer,
                _stream: stream,
            });
            log::info!("Spectrum analysis system setup completed successfully");
        }
        Err(e) => {
            log::warn!("Spectrum analysis setup failed: {}", e);
        }
    }
}

// Helper function to setup audio stream
fn setup_audio_stream() -> Result<
    (
        cpal::Stream,
        ringbuf::wrap::caching::Caching<std::sync::Arc<ringbuf::HeapRb<f32>>, false, true>,
    ),
    Box<dyn std::error::Error>,
> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use ringbuf::traits::{Producer, Split};

    let host = cpal::default_host();
    let input_device = host
        .default_input_device()
        .ok_or("Failed to find input device")?;
    let mut supported_config_range = input_device
        .supported_input_configs()
        .map_err(|e| format!("Error while querying configs: {}", e))?;
    let supported_config = supported_config_range
        .next()
        .ok_or("No supported config")?
        .with_max_sample_rate();
    let config = supported_config.into();

    let ring_buffer = ringbuf::HeapRb::<f32>::new(SPECTRUM_NUM_SAMPLES * 2);
    let (mut prod, cons) = ring_buffer.split();

    // Pre-fill with zeros
    for _ in 0..SPECTRUM_NUM_SAMPLES {
        let _ = prod.try_push(0.0);
    }

    let stream = input_device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for d in data {
                    let _ = prod.try_push(*d);
                }
            },
            move |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        )
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to play stream: {}", e))?;

    Ok((stream, cons))
}

// Update Spectrum history in the materials
fn update_spectrum_system(
    materials: Res<Assets<ShekerShaderMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut spectrum_tracker: ResMut<SpectrumHistoryTracker>,
    quad_query: Query<&MeshMaterial2d<ShekerShaderMaterial>, With<FullscreenQuad>>,
) {
    // Copy config values to avoid borrow conflicts
    let sampling_rate = spectrum_tracker.sampling_rate;
    let min_frequency = spectrum_tracker.min_frequency;
    let max_frequency = spectrum_tracker.max_frequency;

    // Collect samples from audio stream
    let samples: [f32; SPECTRUM_NUM_SAMPLES] = {
        // Check if audio stream is available
        let consumer = match &mut spectrum_tracker.audio_stream {
            Some(stream) => &mut stream.consumer,
            None => return, // No audio stream, skip update
        };

        // Check if we have enough samples for FFT
        use ringbuf::traits::Observer;
        if consumer.occupied_len() < SPECTRUM_NUM_SAMPLES {
            return;
        }

        // Collect samples from ring buffer
        let mut samples = [0.0f32; SPECTRUM_NUM_SAMPLES];
        use ringbuf::traits::Consumer;
        for sample_slot in samples.iter_mut() {
            if let Some(sample) = consumer.try_pop() {
                *sample_slot = sample;
            }
        }
        samples
    };

    // Apply Hann window
    use spectrum_analyzer::windows::hann_window;
    let hann_window = hann_window(&samples);

    // Perform FFT
    use spectrum_analyzer::scaling::divide_by_N_sqrt;
    use spectrum_analyzer::{FrequencyLimit, samples_fft_to_spectrum};

    let spectrum_result = samples_fft_to_spectrum(
        &hann_window,
        sampling_rate,
        FrequencyLimit::Range(min_frequency, max_frequency),
        Some(&divide_by_N_sqrt),
    );

    let spectrum = match spectrum_result {
        Ok(s) => s,
        Err(e) => {
            log::error!("FFT failed: {:?}", e);
            return;
        }
    };

    // Pack frequency and amplitude data into vec4 arrays
    let mut frequencies = [[0.0f32; 4]; SPECTRUM_NUM_SAMPLES / 4];
    let mut amplitudes = [[0.0f32; 4]; SPECTRUM_NUM_SAMPLES / 4];

    for (i, f) in spectrum.data().iter().enumerate() {
        let vec4_index = i / 4;
        let element_index = i % 4;
        frequencies[vec4_index][element_index] = f.0.val();
        amplitudes[vec4_index][element_index] = f.1.val();
    }

    let (max_fr, max_amp) = spectrum.max();

    // Create new frame data
    let new_frame = SpectrumFrameData {
        frequencies,
        amplitudes,
        num_points: spectrum.data().len() as u32,
        max_frequency: max_fr.val(),
        max_amplitude: max_amp.val(),
    };

    // Push current frame to history before updating
    let old_frame = spectrum_tracker.current_frame;
    spectrum_tracker.history.push_back(old_frame);
    // Keep only last 511 frames (plus current = 512 total)
    if spectrum_tracker.history.len() > 511 {
        spectrum_tracker.history.pop_front();
    }

    // Update current frame
    spectrum_tracker.current_frame = new_frame;

    // Prepare GPU buffer data from current frame and history
    let shader_data_vec = spectrum_tracker.prepare_shader_data();
    let data_bytes = bytemuck::cast_slice(&shader_data_vec).to_vec();

    // Update storage buffers for all materials
    for material_handle in quad_query.iter() {
        if let Some(material) = materials.get(&material_handle.0) {
            if let Some(buffer) = storage_buffers.get_mut(&material.spectrum_history) {
                // Directly update bytes to avoid stack overflow
                buffer.data = Some(data_bytes.clone());
            }
        }
    }
}

// Setup OSC input system
fn setup_osc_system(mut osc_tracker: ResMut<OscHistoryTracker>, config: Res<crate::ShekerConfig>) {
    // Check if OSC is configured
    let osc_config = match &config.config.osc {
        Some(cfg) => cfg,
        None => {
            log::info!("OSC input disabled in configuration");
            return;
        }
    };

    log::info!("Setting up OSC input system on port {}", osc_config.port);

    // Build sound map from config
    let mut sound_map = std::collections::HashMap::new();
    for sound in &osc_config.sound {
        sound_map.insert(sound.name.clone(), sound.id);
        log::info!("OSC sound mapping: {} -> {}", sound.name, sound.id);
    }
    osc_tracker.sound_map = sound_map;

    // Start async OSC server
    let port = osc_config.port;
    match start_osc_server(port) {
        Ok(receiver) => {
            osc_tracker.receiver = Some(receiver);
            log::info!("OSC server started successfully on port {}", port);
        }
        Err(e) => {
            log::error!("Failed to start OSC server: {}", e);
        }
    }
}

// Helper function to start OSC server
fn start_osc_server(
    port: u32,
) -> Result<async_std::channel::Receiver<rosc::OscPacket>, Box<dyn std::error::Error>> {
    use async_std::channel::unbounded;
    use async_std::net::{SocketAddrV4, UdpSocket};
    use std::str::FromStr;

    let (sender, receiver) = unbounded();

    // Spawn async task to run OSC server
    std::thread::spawn(move || {
        async_std::task::block_on(async {
            let addr = match SocketAddrV4::from_str(&format!("0.0.0.0:{}", port)) {
                Ok(addr) => addr,
                Err(e) => {
                    log::error!("Invalid OSC address: {}", e);
                    return;
                }
            };

            let sock = match UdpSocket::bind(addr).await {
                Ok(sock) => sock,
                Err(e) => {
                    log::error!("Failed to bind OSC socket: {}", e);
                    return;
                }
            };

            log::info!("OSC server listening on {}", addr);

            let mut buf = [0u8; rosc::decoder::MTU];
            loop {
                match sock.recv_from(&mut buf).await {
                    Ok((size, _addr)) => match rosc::decoder::decode_udp(&buf[..size]) {
                        Ok((_, packet)) => {
                            if sender.send(packet).await.is_err() {
                                log::warn!("OSC receiver disconnected, stopping server");
                                break;
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to decode OSC packet: {:?}", e);
                        }
                    },
                    Err(e) => {
                        log::error!("OSC recv error: {}", e);
                        break;
                    }
                }
            }
        });
    });

    Ok(receiver)
}

// Update OSC history in the materials
fn update_osc_system(
    time: Res<Time>,
    materials: Res<Assets<ShekerShaderMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut osc_tracker: ResMut<OscHistoryTracker>,
    quad_query: Query<&MeshMaterial2d<ShekerShaderMaterial>, With<FullscreenQuad>>,
) {
    let time_delta = time.delta_secs();

    // Process incoming OSC messages
    if let Some(receiver) = &osc_tracker.receiver {
        // Process all pending messages
        while let Ok(packet) = receiver.try_recv() {
            process_osc_packet(&packet, &osc_tracker.sound_map, &osc_tracker.history_data);
        }
    }

    // Apply time decay to TTL values
    {
        let mut history_data = osc_tracker.history_data.lock().unwrap();
        for i in 0..16 {
            let current_ttl = history_data.current_frame.ttls[i];
            let new_ttl = (current_ttl - time_delta).max(0.0);

            if new_ttl <= 0.0 {
                // Clear expired entry
                history_data.update_sound(i, 0);
                history_data.update_ttl(i, 0.0);
                history_data.update_note(i, 0.0);
                history_data.update_gain(i, 0.0);
            } else {
                // Update TTL
                history_data.update_ttl(i, new_ttl);
            }
        }

        // Push current frame to ring buffer
        history_data.push_current_frame();
    }

    // Prepare shader data
    let shader_data_vec = {
        let history_data = osc_tracker.history_data.lock().unwrap();
        history_data.prepare_shader_data()
    };

    // Create array from vec
    let mut history_array = [OscShaderData {
        sounds: [[0; 4]; 4],
        ttls: [[0.0; 4]; 4],
        notes: [[0.0; 4]; 4],
        gains: [[0.0; 4]; 4],
    }; 512];
    history_array.copy_from_slice(&shader_data_vec[..512]);

    // Create GPU buffer data
    let buffer_data = OscHistoryBuffer {
        history_data: history_array,
    };

    // Update storage buffers for all materials
    for material_handle in quad_query.iter() {
        if let Some(material) = materials.get(&material_handle.0) {
            if let Some(buffer) = storage_buffers.get_mut(&material.osc_history) {
                buffer.set_data(buffer_data);
            }
        }
    }
}

// Process OSC packet and update history
fn process_osc_packet(
    packet: &rosc::OscPacket,
    sound_map: &std::collections::HashMap<String, i32>,
    history_data: &std::sync::Arc<std::sync::Mutex<OscHistoryData>>,
) {
    use rosc::{OscPacket, OscType};

    match packet {
        OscPacket::Message(msg) => {
            // Parse OSC message parameters
            let mut id: usize = 0;
            let mut ttl = 0.0;
            let mut note = 0.0;
            let mut gain = 0.0;
            let mut sound = 0;

            for (i, v) in msg.args.iter().enumerate() {
                if let OscType::String(val) = v {
                    match val.as_str() {
                        "orbit" => {
                            if let Some(OscType::Int(orbit)) = msg.args.get(i + 1) {
                                id = *orbit as usize;
                            }
                        }
                        "delta" => {
                            if let Some(OscType::Float(delta)) = msg.args.get(i + 1) {
                                ttl = *delta;
                            }
                        }
                        "note" | "n" => {
                            if let Some(OscType::Float(n)) = msg.args.get(i + 1) {
                                note = *n;
                            }
                        }
                        "gain" => {
                            if let Some(OscType::Float(g)) = msg.args.get(i + 1) {
                                gain = *g;
                            }
                        }
                        "sound" | "s" => {
                            if let Some(OscType::String(s)) = msg.args.get(i + 1) {
                                if let Some(&sound_id) = sound_map.get(s.as_str()) {
                                    sound = sound_id;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Update values if id is within bounds
            if id < 16 {
                let mut history_data = history_data.lock().unwrap();
                history_data.update_sound(id, sound);
                history_data.update_ttl(id, ttl);
                history_data.update_note(id, note);
                history_data.update_gain(id, gain);
            }
        }
        OscPacket::Bundle(bundle) => {
            // Process first message in bundle (matching original behavior)
            if let Some(OscPacket::Message(msg)) = bundle.content.first() {
                process_osc_packet(&OscPacket::Message(msg.clone()), sound_map, history_data);
            }
        }
    }
}

// Generate dynamic shader file using ShaderPreprocessor
fn generate_dynamic_shader_file(
    config: &crate::ShekerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
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

    log::info!(
        "Dynamic shader file generated successfully at {}",
        output_path
    );
    Ok(())
}

// Generate shader using ShaderPreprocessor
fn generate_shader_with_preprocessor(
    config: &crate::ShekerConfig,
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

// Generate shader source for a specific pass in multi-pass rendering
fn generate_shader_for_pass(
    config: &crate::ShekerConfig,
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
    let is_multipass = config.config.pipeline.len() > 1;
    let enable_texture_sampling = is_multipass && pass_index > 0;

    log::info!(
        "Pass {}: uses_texture={}, enable_texture_sampling={}, is_multipass={}",
        pass_index,
        uses_texture,
        enable_texture_sampling,
        is_multipass
    );

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
        log::info!("Pass {}: Including mouse input module", pass_index);
    }

    if uses_spectrum {
        let spectrum_wgsl = include_str!("../shaders/spectrum.wgsl");
        shader_parts.push("// === SPECTRUM DEFINITIONS ===".to_string());
        shader_parts.push(spectrum_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including spectrum analysis module", pass_index);
    }

    if uses_osc {
        let osc_wgsl = include_str!("../shaders/osc.wgsl");
        shader_parts.push("// === OSC DEFINITIONS ===".to_string());
        shader_parts.push(osc_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including OSC input module", pass_index);
    }

    if uses_midi {
        let midi_wgsl = include_str!("../shaders/midi.wgsl");
        shader_parts.push("// === MIDI DEFINITIONS ===".to_string());
        shader_parts.push(midi_wgsl.to_string());
        shader_parts.push("".to_string());
        log::info!("Pass {}: Including MIDI input module", pass_index);
    }

    // Include texture module if this pass needs to sample from previous pass
    if enable_texture_sampling || uses_texture {
        let texture_wgsl = include_str!("../shaders/texture.wgsl");
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
fn generate_clean_shader_source(
    config: &crate::ShekerConfig,
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

    // Replace function name and fix coordinate usage
    // In Bevy Material2d, mesh.position is fragment coordinates relative to the mesh,
    // not window coordinates. We need to use mesh.uv * Window.resolution instead.
    let processed_shader = fragment_source
        .replace("fn fs_main(", "fn fragment(")
        .replace("in.position.xy", "(in.uv * Window.resolution)")
        .replace("mesh.position.xy", "(mesh.uv * Window.resolution)");

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
