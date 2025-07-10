struct WindowUniform {
    resolution: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> window: WindowUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(model.position, 1.0);
    // Convert from NDC (-1..1) to UV coordinates (0..1)
    out.tex_coords = (model.position.xy + 1.0) * 0.5;
    return out;
}
