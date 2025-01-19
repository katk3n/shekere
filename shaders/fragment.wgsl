struct WindowUniform {
    resolution: vec2<f32>,
};

struct TimeUniform {
    duration: f32,
};

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

fn colorPalette(t: f32) -> vec3<f32> {
    let a = vec3(0.5, 0.5, 0.5);
    let b = vec3(0.5, 0.5, 0.5);
    let c = vec3(1.0, 1.0, 1.0);
    let d = vec3(0.00, 0.10, 0.20);
    return a + b * cos(6.28318 * (c * t + d));
}

fn to_linear_rgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = (in.position.xy * 2.0 - window.resolution.xy) / min(window.resolution.x, window.resolution.y);
    var uv0 = uv;
    var finalColor = vec3(0.0);

    for (var i: i32 = 0; i < 4; i++) {
        uv = fract(uv * 1.5) - 0.5;

        var d = length(uv) * exp(-length(uv0));
        var index = f32(i);
        var col = colorPalette(length(uv0) + index * 0.4 + time.duration * 0.4);

        d = sin(d * 8.0 + time.duration) / 8.0;
        d = abs(d);
        d = pow(0.01 / d, 1.2);

        finalColor += col * d;
    }

    return vec4<f32>(to_linear_rgb(finalColor), 1.0);
}

