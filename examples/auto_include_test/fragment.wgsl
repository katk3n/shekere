// No explicit includes needed - common definitions are automatically included!

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = normalized_coords(in.position.xy);
    let m = mouse_coords();
    
    // Create a gradient based on time and mouse position
    let color = vec3(
        0.5 + 0.5 * sin(time.duration + uv.x * 3.0),
        0.5 + 0.5 * cos(time.duration + uv.y * 3.0 + m.x),
        0.5 + 0.5 * sin(time.duration * 0.7 + length(uv) * 5.0 + m.y)
    );
    
    // Use the built-in color conversion function
    return vec4(to_linear_rgb(color), 1.0);
}