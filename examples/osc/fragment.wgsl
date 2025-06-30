// Common definitions (including OscTruck, OscUniform, and bindings) are automatically included

fn orb(p: vec2<f32>, p0: vec2<f32>, r: f32, col: vec3<f32>) -> vec3<f32> {
    var t = clamp(1.0 + r - length(p - p0), 0.0, 1.0);
    return vec3(pow(t, 16.0) * col);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let PI = 3.14159265;
    let uv = NormalizedCoords(in.position.xy);

    let white = vec3(1.0, 1.0, 1.0);
    let black = vec3(0.0, 0.0, 0.0);
    let red = vec3(1.0, 0.0, 0.0);
    let blue = vec3(0.0, 0.0, 1.0);

    var gain1 = 0.0;  // gain of d1
    var gain2 = 0.0;  // gain of d2
    var gain3 = 0.0;  // gain of d3

    if Osc.trucks[0].sound == 1 {  // bd
        gain1 = Osc.trucks[0].gain * Osc.trucks[0].ttl * 0.1;
    }
    if Osc.trucks[1].sound == 2 {  // sd
        gain2 = Osc.trucks[1].gain * Osc.trucks[1].ttl * 0.1;
    }
    if Osc.trucks[2].sound == 3 {  // hc
        gain3 = Osc.trucks[2].gain * Osc.trucks[2].ttl * 0.1;
    }

    let v = 0.3;
    let d = 0.7;

    var p1 = vec2(0.0, 0.0);
    var p2 = vec2(
        d * cos(Time.duration * v * PI),
        d * sin(Time.duration * v * PI)
    );
    var p3 = vec2(
        d * cos(Time.duration * v * PI + PI),
        d * sin(Time.duration * v * PI + PI)
    );

    var col = black;
    col += orb(uv, p1, 0.15 + gain1, white);
    col += orb(uv, p2, 0.08 + gain2, red);
    col += orb(uv, p3, 0.08 + gain3, blue);

    return vec4(ToLinearRgb(col), 1.0);
}
