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
};

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;
@group(1) @binding(0) var<uniform> mouse: MouseUniform;
@group(2) @binding(0) var<uniform> osc: OscUniform;

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
    let PI = 3.14159265;
    let min_xy = min(window.resolution.x, window.resolution.y);
    let uv = (in.position.xy * 2.0 - window.resolution) / min_xy;

    let white = vec3(1.0, 1.0, 1.0);
    let black = vec3(0.0, 0.0, 0.0);
    let red = vec3(1.0, 0.0, 0.0);
    let blue = vec3(0.0, 0.0, 1.0);

    var gain1 = 0.0;  // gain of d1
    var gain2 = 0.0;  // gain of d2
    var gain3 = 0.0;  // gain of d3

    if osc.trucks[0].sound == 1 {  // bd
        gain1 = osc.trucks[0].gain * osc.trucks[0].ttl * 0.1;
    }
    if osc.trucks[1].sound == 2 {  // sd
        gain2 = osc.trucks[1].gain * osc.trucks[1].ttl * 0.1;
    }
    if osc.trucks[2].sound == 3 {  // hc
        gain3 = osc.trucks[2].gain * osc.trucks[2].ttl * 0.1;
    }

    let v = 0.3;
    let d = 0.7;

    var p1 = vec2(0.0, 0.0);
    var p2 = vec2(
        d * cos(time.duration * v * PI),
        d * sin(time.duration * v * PI)
    );
    var p3 = vec2(
        d * cos(time.duration * v * PI + PI),
        d * sin(time.duration * v * PI + PI)
    );

    var col = black;
    col += orb(uv, p1, 0.15 + gain1, white);
    col += orb(uv, p2, 0.08 + gain2, red);
    col += orb(uv, p3, 0.08 + gain3, blue);

    return vec4(to_linear_rgb(col), 1.0);
}
