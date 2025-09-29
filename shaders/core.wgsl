// Core definitions for shekere shaders
// Contains basic uniform structures and utility functions

// === UNIFORM STRUCTURES ===

struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
}

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
}

// === UNIFORM BINDINGS ===

// Group 2: Material uniforms (Bevy Material2d compatibility)
@group(2) @binding(0) var<uniform> Window: WindowUniform;
@group(2) @binding(1) var<uniform> Time: TimeUniform;

// === UTILITY FUNCTIONS ===

// Color space conversion
fn ToLinearRgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

// Coordinate system helpers
fn NormalizedCoords(position: vec2<f32>) -> vec2<f32> {
    let min_xy = min(Window.resolution.x, Window.resolution.y);
    return (position * 2.0 - Window.resolution) / min_xy;
}