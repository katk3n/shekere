struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
};

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
};

struct MouseUniform {
    // mouse position in physical size
    position: vec2<f32>,
};

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;
@group(1) @binding(0) var<uniform> mouse: MouseUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

fn to_linear_rgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

fn orb(p: vec2<f32>, p0: vec2<f32>, r: f32, col: vec3<f32>) -> vec3<f32> {
    var t = clamp(1.0 + r - length(p - p0), 0.0, 1.0);
    return vec3(pow(t, 16.0) * col);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let min_xy = min(window.resolution.x, window.resolution.y);
    let uv = (in.position.xy * 2.0 - window.resolution) / min_xy;
    let m = (mouse.position * 2.0 - window.resolution) / min_xy;

    let green = vec3(0.0, 1.0, 0.0);
    let black = vec3(0.0, 0.0, 0.0);

    var col = black;
    col += orb(uv, m, 0.07, green);

    return vec4(to_linear_rgb(col), 1.0);
}
