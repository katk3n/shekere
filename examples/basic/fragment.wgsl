@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = NormalizedCoords(in.position.xy);
    
    let color = vec3(
        sin(Time.duration + uv.x) * 0.5 + 0.5,
        cos(Time.duration + uv.y) * 0.5 + 0.5,
        sin(Time.duration + length(uv)) * 0.5 + 0.5
    );
    
    return vec4(ToLinearRgb(color), 1.0);
}