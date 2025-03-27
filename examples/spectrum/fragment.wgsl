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
    data_points: array<SpectrumDataPoint, 2048>,
    num_frequencies: u32,
    max_frequency: f32,
    max_amplitude: f32,
}

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;
@group(1) @binding(0) var<uniform> mouse: MouseUniform;
@group(2) @binding(1) var<uniform> audio: SpectrumUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}
;

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
    let PI = 3.14159265;
    let min_xy = min(window.resolution.x, window.resolution.y);
    let uv = (in.position.xy * 2.0 - window.resolution) / min_xy;

    let white = vec3(1.0, 1.0, 1.0);
    let black = vec3(0.0, 0.0, 0.0);
    let red = vec3(1.0, 0.0, 0.0);
    let blue = vec3(0.0, 0.0, 1.0);

    var gain1 = 0.0; // gain of d1
    var gain2 = 0.0; // gain of d2
    var gain3 = 0.0; // gain of d3

    if audio.max_frequency < 100.0 {
        gain2 = gain2 + audio.max_amplitude;
    } else if audio.max_frequency > 200.0 {
        gain3 = gain3 + audio.max_amplitude;
    } else {
        gain1 = gain1 + audio.max_amplitude;
    }

    let v = 0.3;
    let d = 0.7;

    var p1 = vec2(0.0, 0.0);
    var p2 = vec2(d * cos(time.duration * v * PI), d * sin(time.duration * v * PI));
    var p3 = vec2(d * cos(time.duration * v * PI + PI), d * sin(time.duration * v * PI + PI));

    var col = black;
    col += orb(uv, p1, 0.15 + gain1, white);
    col += orb(uv, p2, 0.08 + gain2, red);
    col += orb(uv, p3, 0.08 + gain3, blue);

    return vec4(to_linear_rgb(col), 1.0);
}
