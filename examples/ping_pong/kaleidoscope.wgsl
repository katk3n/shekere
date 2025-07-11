@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    let center = vec2<f32>(0.5, 0.5);
    let centered_uv = uv - center;
    
    // Create kaleidoscope effect by sampling multiple symmetric positions
    let time = Time.duration * 0.5;
    let rotation_angle = time * 0.2;
    
    // Rotation matrix
    let cos_rot = cos(rotation_angle);
    let sin_rot = sin(rotation_angle);
    let rotation_matrix = mat2x2<f32>(
        cos_rot, -sin_rot,
        sin_rot, cos_rot
    );
    
    // Sample previous frame with various transformations for kaleidoscope effect
    var kaleidoscope_color = vec4<f32>(0.0);
    let num_segments = 6.0; // Number of kaleidoscope segments
    
    for (var i = 0; i < 6; i = i + 1) {
        let angle = f32(i) * 2.0 * 3.14159 / num_segments;
        let cos_a = cos(angle);
        let sin_a = sin(angle);
        
        // Rotation matrix for this segment
        let segment_matrix = mat2x2<f32>(
            cos_a, -sin_a,
            sin_a, cos_a
        );
        
        // Apply rotation and reflection
        var sample_uv = segment_matrix * rotation_matrix * centered_uv;
        
        // Create reflection effect
        if (f32(i) % 2.0 < 1.0) {
            sample_uv.x = -sample_uv.x;
        }
        
        // Map back to texture coordinates
        sample_uv = sample_uv + center;
        
        // Sample only if within bounds
        if (sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && 
            sample_uv.y >= 0.0 && sample_uv.y <= 1.0) {
            kaleidoscope_color += SamplePreviousPass(sample_uv);
        }
    }
    
    // Average the kaleidoscope samples
    kaleidoscope_color = kaleidoscope_color / num_segments;
    
    // Apply feedback decay to prevent infinite accumulation
    let feedback_strength = 0.85;
    kaleidoscope_color *= feedback_strength;
    
    // Generate new content - a simple radial pattern
    let distance_from_center = length(centered_uv);
    let angle_from_center = atan2(centered_uv.y, centered_uv.x);
    
    // Create a moving spiral pattern
    let spiral_pattern = sin(distance_from_center * 20.0 - time * 3.0 + angle_from_center * 3.0);
    let radial_pattern = cos(angle_from_center * 8.0 + time * 2.0);
    
    // Create color based on position and time
    var new_content = vec4<f32>(
        0.5 + 0.3 * sin(time + distance_from_center * 5.0),
        0.5 + 0.3 * cos(time * 1.3 + angle_from_center),
        0.5 + 0.3 * sin(time * 0.7 + spiral_pattern),
        1.0
    );
    
    // Apply intensity based on patterns
    let pattern_intensity = smoothstep(0.7, 0.9, spiral_pattern * radial_pattern);
    new_content *= pattern_intensity * 0.1; // Keep new content subtle
    
    // Combine kaleidoscope feedback with new content
    let result = kaleidoscope_color + new_content;
    
    return vec4<f32>(
        clamp(result.r, 0.0, 1.0),
        clamp(result.g, 0.0, 1.0),
        clamp(result.b, 0.0, 1.0),
        1.0
    );
}