@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    
    // Simple box blur effect - much larger blur size for clearly visible effect
    let blur_size = 15.0 / Window.resolution;
    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    
    // 7x7 blur kernel for stronger effect
    for (var x = -3; x <= 3; x++) {
        for (var y = -3; y <= 3; y++) {
            let offset = vec2<f32>(f32(x), f32(y)) * blur_size;
            let sample_uv = uv + offset;
            
            // Only sample if within bounds
            if (sample_uv.x >= 0.0 && sample_uv.x <= 1.0 && 
                sample_uv.y >= 0.0 && sample_uv.y <= 1.0) {
                let weight = 1.0;
                color += SamplePreviousPass(sample_uv) * weight;
                total_weight += weight;
            }
        }
    }
    
    if (total_weight > 0.0) {
        color /= total_weight;
    }
    
    // Add a slight color enhancement
    let enhanced_rgb = pow(color.rgb, vec3<f32>(0.9));
    
    return vec4<f32>(enhanced_rgb, color.a);
}