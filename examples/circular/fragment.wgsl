@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = NormalizedCoords(in.position.xy);
    
    // Create concentric circles
    let dist = length(uv);
    let rings = sin(dist * 10.0 - Time.duration * 3.0) * 0.5 + 0.5;
    
    let color = vec3(rings);
    
    return vec4(ToLinearRgb(color), 1.0);
}