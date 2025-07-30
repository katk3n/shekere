// Embedded common definitions for shekere shaders
// This file is automatically included at the beginning of every shader

// === UNIFORM STRUCTURES ===

struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
}

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
}

struct MouseUniform {
    // mouse position in physical size
    position: vec2<f32>,
}


struct SpectrumUniform {
    // frequency values of audio input (packed into vec4s for alignment)
    frequencies: array<vec4<f32>, 512>,
    // amplitude values of audio input (packed into vec4s for alignment)
    amplitudes: array<vec4<f32>, 512>,
    // the number of data points
    num_points: u32,
    // frequency of the data point with the max amplitude
    max_frequency: f32,
    // max amplitude of audio input
    max_amplitude: f32,
    _padding: u32,
}

struct OscUniform {
    // OSC sound values (packed into vec4s for alignment)
    sounds: array<vec4<i32>, 4>,
    // OSC ttl values (packed into vec4s for alignment)
    ttls: array<vec4<f32>, 4>,
    // OSC note values (packed into vec4s for alignment)
    notes: array<vec4<f32>, 4>,
    // OSC gain values (packed into vec4s for alignment)
    gains: array<vec4<f32>, 4>,
}

struct MidiUniform {
    // note velocities (0-127 normalized to 0.0-1.0) 
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    notes: array<vec4<f32>, 32>,
    // control change values (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    controls: array<vec4<f32>, 32>,
    // note on attack detection (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    note_on: array<vec4<f32>, 32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

// === UNIFORM BINDINGS ===

// Group 0: Always available uniforms
@group(0) @binding(0) var<uniform> Window: WindowUniform;
@group(0) @binding(1) var<uniform> Time: TimeUniform;

// Group 1: Device uniforms (conditional)
@group(1) @binding(0) var<uniform> Mouse: MouseUniform;

// Group 2: Sound uniforms (conditional - only bind what you use)
@group(2) @binding(0) var<uniform> Osc: OscUniform;
@group(2) @binding(1) var<uniform> Spectrum: SpectrumUniform;
@group(2) @binding(2) var<uniform> Midi: MidiUniform;

// Group 3: Multi-pass textures (conditional - only available in multi-pass shaders)
@group(3) @binding(0) var previous_pass: texture_2d<f32>;
@group(3) @binding(1) var texture_sampler: sampler;

// === UTILITY FUNCTIONS ===

// Spectrum data access helpers
fn SpectrumFrequency(index: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;
    
    if vec4_index >= 512u {
        return 0.0;
    }
    
    let freq_vec = Spectrum.frequencies[vec4_index];
    switch element_index {
        case 0u: { return freq_vec.x; }
        case 1u: { return freq_vec.y; }
        case 2u: { return freq_vec.z; }
        case 3u: { return freq_vec.w; }
        default: { return 0.0; }
    }
}

fn SpectrumAmplitude(index: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;
    
    if vec4_index >= 512u {
        return 0.0;
    }
    
    let amp_vec = Spectrum.amplitudes[vec4_index];
    switch element_index {
        case 0u: { return amp_vec.x; }
        case 1u: { return amp_vec.y; }
        case 2u: { return amp_vec.z; }
        case 3u: { return amp_vec.w; }
        default: { return 0.0; }
    }
}

// OSC data access helpers
fn OscSound(index: u32) -> i32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;
    
    if vec4_index >= 4u {
        return 0;
    }
    
    let sound_vec = Osc.sounds[vec4_index];
    switch element_index {
        case 0u: { return sound_vec.x; }
        case 1u: { return sound_vec.y; }
        case 2u: { return sound_vec.z; }
        case 3u: { return sound_vec.w; }
        default: { return 0; }
    }
}

fn OscTtl(index: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;
    
    if vec4_index >= 4u {
        return 0.0;
    }
    
    let ttl_vec = Osc.ttls[vec4_index];
    switch element_index {
        case 0u: { return ttl_vec.x; }
        case 1u: { return ttl_vec.y; }
        case 2u: { return ttl_vec.z; }
        case 3u: { return ttl_vec.w; }
        default: { return 0.0; }
    }
}

fn OscNote(index: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;
    
    if vec4_index >= 4u {
        return 0.0;
    }
    
    let note_vec = Osc.notes[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}

fn OscGain(index: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;
    
    if vec4_index >= 4u {
        return 0.0;
    }
    
    let gain_vec = Osc.gains[vec4_index];
    switch element_index {
        case 0u: { return gain_vec.x; }
        case 1u: { return gain_vec.y; }
        case 2u: { return gain_vec.z; }
        case 3u: { return gain_vec.w; }
        default: { return 0.0; }
    }
}

// Color space conversion
fn ToLinearRgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

fn ToSrgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 1.0 / 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

// Coordinate system helpers
fn NormalizedCoords(position: vec2<f32>) -> vec2<f32> {
    let min_xy = min(Window.resolution.x, Window.resolution.y);
    return (position * 2.0 - Window.resolution) / min_xy;
}

fn MouseCoords() -> vec2<f32> {
    return NormalizedCoords(Mouse.position);
}

// MIDI helper functions
fn MidiNote(note_num: u32) -> f32 {
    let vec4_index = note_num / 4u;
    let element_index = note_num % 4u;
    
    if vec4_index >= 32u {
        return 0.0;
    }
    
    let note_vec = Midi.notes[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}

fn MidiControl(cc_num: u32) -> f32 {
    let vec4_index = cc_num / 4u;
    let element_index = cc_num % 4u;
    
    if vec4_index >= 32u {
        return 0.0;
    }
    
    let cc_vec = Midi.controls[vec4_index];
    switch element_index {
        case 0u: { return cc_vec.x; }
        case 1u: { return cc_vec.y; }
        case 2u: { return cc_vec.z; }
        case 3u: { return cc_vec.w; }
        default: { return 0.0; }
    }
}

fn MidiNoteOn(note_num: u32) -> f32 {
    let vec4_index = note_num / 4u;
    let element_index = note_num % 4u;
    
    if vec4_index >= 32u {
        return 0.0;
    }
    
    let note_vec = Midi.note_on[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}

// Multi-pass texture helper functions
fn SamplePreviousPass(uv: vec2<f32>) -> vec4<f32> {
    // Fix Y-axis flipping for persistent textures
    let corrected_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return textureSample(previous_pass, texture_sampler, corrected_uv);
}

fn SamplePreviousPassOffset(uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32> {
    return textureSample(previous_pass, texture_sampler, uv + offset);
}