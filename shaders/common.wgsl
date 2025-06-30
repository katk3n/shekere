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
@group(0) @binding(0) var<uniform> Window: WindowUniform;
@group(0) @binding(1) var<uniform> Time: TimeUniform;

// Group 1: Device uniforms (conditional)
@group(1) @binding(0) var<uniform> Mouse: MouseUniform;

// Group 2: Sound uniforms (conditional - only bind what you use)
@group(2) @binding(0) var<uniform> Osc: OscUniform;
@group(2) @binding(1) var<uniform> Spectrum: SpectrumUniform;
@group(2) @binding(2) var<uniform> Midi: MidiUniform;

// === UTILITY FUNCTIONS ===

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