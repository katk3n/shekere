// Common definitions (including SpectrumUniform and bindings) are automatically included

fn bar(uv: vec2<f32>, x: f32, width: f32, height: f32) -> bool {
    if (uv.x > x) && (uv.x < x + width) && (abs(uv.y) < height) {
        return true;
    }
    return false;
}

fn hue_to_rgb(hue: f32) -> vec3<f32> {
    let kr = (5.0 + hue * 6.0) % 6.0;
    let kg = (3.0 + hue * 6.0) % 6.0;
    let kb = (1.0 + hue * 6.0) % 6.0;

    let r = 1.0 - max(min(min(kr, 4.0 - kr), 1.0), 0.0);
    let g = 1.0 - max(min(min(kg, 4.0 - kg), 1.0), 0.0);
    let b = 1.0 - max(min(min(kb, 4.0 - kb), 1.0), 0.0);

    return vec3(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let min_xy = min(Window.resolution.x, Window.resolution.y);
    let uv = vec2(in.position.x / min_xy,(1.0 - in.position.y / min_xy) * 2.0 - 1.0);

    let num_steps = Spectrum.num_points;
    let width = 1.0 / f32(num_steps) / 2.0;
    let max_hue = 0.7;

    var col = vec3(0.0);
    for (var i = 0u; i < num_steps; i++) {
        let height = SpectrumAmplitude(i);
        if bar(uv, f32(i) / f32(num_steps), width, height) {
            col = hue_to_rgb(max_hue * f32(i) / f32(num_steps));
            break;
        }
    }

    // draw horizontal line
    if abs(uv.y) < 0.001 {
        col = hue_to_rgb(max_hue * uv.x);
    }

    return vec4(ToLinearRgb(col), 1.0);
}
