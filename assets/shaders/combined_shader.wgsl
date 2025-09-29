#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct WindowUniform {
    resolution: vec2<f32>,
}

struct TimeUniform {
    time: f32,
}

// Basic uniforms from material
@group(2) @binding(0) var<uniform> Window: WindowUniform;
@group(2) @binding(1) var<uniform> Time: TimeUniform;

// Helper functions
fn NormalizedCoords(position: vec2<f32>) -> vec2<f32> {
    let min_xy = min(Window.resolution.x, Window.resolution.y);
    return (position * 2.0 - Window.resolution) / min_xy;
}

fn ToLinearRgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let screen_uv = NormalizedCoords(mesh.uv * Window.resolution);

    let color = vec3(
        sin(Time.time + screen_uv.x) * 0.5 + 0.5,
        cos(Time.time + screen_uv.y) * 0.5 + 0.5,
        sin(Time.time + length(screen_uv)) * 0.5 + 0.5
    );

    return vec4(ToLinearRgb(color), 1.0);
}