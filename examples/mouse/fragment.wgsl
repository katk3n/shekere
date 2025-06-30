// Common definitions (WindowUniform, TimeUniform, etc.) are automatically included

fn orb(p: vec2<f32>, p0: vec2<f32>, r: f32, col: vec3<f32>) -> vec3<f32> {
    var t = clamp(1.0 + r - length(p - p0), 0.0, 1.0);
    return vec3(pow(t, 16.0) * col);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = normalized_coords(in.position.xy);
    let m = mouse_coords();

    let green = vec3(0.0, 1.0, 0.0);
    let black = vec3(0.0, 0.0, 0.0);

    var col = black;
    col += orb(uv, m, 0.07, green);

    return vec4(to_linear_rgb(col), 1.0);
}
