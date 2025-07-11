// Scene shader - first pass, doesn't use previous_pass texture
@fragment  
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    let center = vec2<f32>(0.5, 0.5);
    let distance = length(uv - center);
    
    // Create a colorful animated pattern
    let time_offset = Time.duration * 0.5;
    let spiral = sin(distance * 20.0 - time_offset) * 0.5 + 0.5;
    let rings = sin(distance * 40.0) * 0.5 + 0.5;
    
    // Rainbow colors based on angle
    let angle = atan2(uv.y - center.y, uv.x - center.x);
    let normalized_angle = (angle + 3.14159) / (2.0 * 3.14159);
    
    let red = sin(normalized_angle * 6.28318 + time_offset) * 0.5 + 0.5;
    let green = sin(normalized_angle * 6.28318 + time_offset + 2.094) * 0.5 + 0.5;
    let blue = sin(normalized_angle * 6.28318 + time_offset + 4.188) * 0.5 + 0.5;
    
    let color = vec3<f32>(red, green, blue) * spiral * rings;
    
    // Fade edges
    let edge_fade = smoothstep(0.4, 0.0, distance);
    
    return vec4<f32>(color * edge_fade, 1.0);
}