@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = position.xy / Window.resolution;
    let color = vec3<f32>(uv, 0.5 + 0.5 * sin(Time.duration));
    return vec4<f32>(color, 1.0);
}