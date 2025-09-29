// Multi-pass texture definitions for shekere shaders

// === TEXTURE BINDINGS ===

// Group 2: Multi-pass textures (conditional - only available in multi-pass shaders)
@group(2) @binding(6) var previous_pass: texture_2d<f32>;
@group(2) @binding(7) var texture_sampler: sampler;

// === TEXTURE FUNCTIONS ===

// Multi-pass texture helper functions
fn SamplePreviousPass(uv: vec2<f32>) -> vec4<f32> {
    // Fix Y-axis flipping for persistent textures
    let corrected_uv = vec2<f32>(uv.x, 1.0 - uv.y);
    return textureSample(previous_pass, texture_sampler, corrected_uv);
}

fn SamplePreviousPassOffset(uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32> {
    return textureSample(previous_pass, texture_sampler, uv + offset);
}