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

struct SpectrumDataPoint {
    frequency: f32,
    amplitude: f32,
    _padding: vec2<u32>,
}

struct SpectrumUniform {
    // spectrum data points of audio input
    data_points: array<SpectrumDataPoint, 2048>,
    // the number of data points
    num_points: u32,
    // frequency of the data point with the max amplitude
    max_frequency: f32,
    // max amplitude of audio input
    max_amplitude: f32,
}

struct OscTruck {
    // OSC parameters for each OSC truck
    sound: i32,
    ttl: f32,
    note: f32,
    gain: f32,
}

struct OscUniform {
    // OSC trucks (d1-d16), osc[0] for OSC d1
    trucks: array<OscTruck, 16>,
}

struct MidiUniform {
    // note velocities (0-127 normalized to 0.0-1.0) 
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    notes: array<vec4<f32>, 32>,
    // control change values (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    controls: array<vec4<f32>, 32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

// === UNIFORM BINDINGS ===

// Group 0: Always available uniforms
@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;

// Group 1: Device uniforms (conditional)
@group(1) @binding(0) var<uniform> mouse: MouseUniform;

// Group 2: Sound uniforms (conditional - only bind what you use)
@group(2) @binding(0) var<uniform> osc: OscUniform;
@group(2) @binding(1) var<uniform> spectrum: SpectrumUniform;
@group(2) @binding(2) var<uniform> midi: MidiUniform;

// === UTILITY FUNCTIONS ===

// Color space conversion
fn to_linear_rgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

fn to_srgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 1.0 / 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

// Coordinate system helpers
fn normalized_coords(position: vec2<f32>) -> vec2<f32> {
    let min_xy = min(window.resolution.x, window.resolution.y);
    return (position * 2.0 - window.resolution) / min_xy;
}

fn mouse_coords() -> vec2<f32> {
    return normalized_coords(mouse.position);
}

// MIDI helper functions
fn midi_note(note_num: u32) -> f32 {
    let vec4_index = note_num / 4u;
    let element_index = note_num % 4u;
    
    if vec4_index >= 32u {
        return 0.0;
    }
    
    let note_vec = midi.notes[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}

fn midi_control(cc_num: u32) -> f32 {
    let vec4_index = cc_num / 4u;
    let element_index = cc_num % 4u;
    
    if vec4_index >= 32u {
        return 0.0;
    }
    
    let cc_vec = midi.controls[vec4_index];
    switch element_index {
        case 0u: { return cc_vec.x; }
        case 1u: { return cc_vec.y; }
        case 2u: { return cc_vec.z; }
        case 3u: { return cc_vec.w; }
        default: { return 0.0; }
    }
}