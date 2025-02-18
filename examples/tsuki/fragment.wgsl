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

fn orb(p: vec2<f32>, p0: vec2<f32>, r1: f32, r2: f32, r3: f32, col: vec3<f32>) -> vec3<f32> {
    var t = clamp(r1 - length(p - p0), 0.0, 1.0);
    t = pow(t, sin(time.duration * 2.0) * r2 + r3);
    return vec3(t * col);
}

fn rand(seed: f32) -> f32 {
    return (fract(sin(seed) * 43758.5453) - 0.5) * 2.0;
}

fn rand_position(x: f32) -> vec2<f32> {
    return vec2(rand(x), rand(x * 1.1)) * 0.9;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let min_xy = min(window.resolution.x, window.resolution.y);
    let m = (mouse.position * 2.0 - window.resolution) / min_xy;
    let p = (in.position.xy * 2.0 - window.resolution) / min_xy;

    let white = vec3(1.0, 1.0, 1.0);
    let bg_color = vec3(0.0, 0.0, 0.12);

    var t = bg_color;
    t += orb(m, p, 1.05, 0.0, 20.0, white);

    for (var i = 0; i < 20; i++) {
        var pos = rand_position(f32(i));
        if pos.x < -0.7 { pos.x += 0.3; }
        if pos.y < -0.7 { pos.y += 0.3; }

        let r = rand(f32(i + 1)) + 0.1;
        let g = rand(f32(i + 2)) + 0.1;
        let b = rand(f32(i + 3)) + 0.1;
        t += orb(pos, p, 1.0, 3.0, 15.0, vec3(r, g, b));
    }

    t += orb(vec2(-0.9, -0.9), p, 1.3, 0.0, 8.0, vec3(1.0, 1.0, 0.3));

    return vec4(to_linear_rgb(t), 1.0);
}
