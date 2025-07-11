@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    let resolution = Window.resolution;
    let texel_size = 1.0 / resolution;
    
    // Reaction-diffusion parameters (Gray-Scott model)
    let feed_rate = 0.055;  // Feed rate for chemical A
    let kill_rate = 0.062;  // Kill rate for chemical B
    let diffusion_a = 1.0;  // Diffusion rate for chemical A
    let diffusion_b = 0.5;  // Diffusion rate for chemical B
    let dt = 1.0;           // Time step
    
    // Sample the previous frame state
    let prev_state = SamplePreviousPass(uv);
    let a = prev_state.r;  // Chemical A concentration
    let b = prev_state.g;  // Chemical B concentration
    
    // Calculate Laplacian (diffusion) using a 3x3 kernel
    var laplacian_a = 0.0;
    var laplacian_b = 0.0;
    
    // 3x3 Laplacian kernel weights
    let center_weight = -1.0;
    let adjacent_weight = 0.2;
    let diagonal_weight = 0.05;
    
    // Apply convolution kernel
    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            let offset = vec2<f32>(f32(dx), f32(dy)) * texel_size;
            let sample_uv = uv + offset;
            let neighbor_state = SamplePreviousPass(sample_uv);
            
            var weight = 0.0;
            if (dx == 0 && dy == 0) {
                weight = center_weight;
            } else if (abs(dx) + abs(dy) == 1) {
                weight = adjacent_weight;
            } else {
                weight = diagonal_weight;
            }
            
            laplacian_a += neighbor_state.r * weight;
            laplacian_b += neighbor_state.g * weight;
        }
    }
    
    // Gray-Scott reaction-diffusion equations
    let reaction = a * b * b;
    let new_a = a + dt * (diffusion_a * laplacian_a - reaction + feed_rate * (1.0 - a));
    let new_b = b + dt * (diffusion_b * laplacian_b + reaction - (kill_rate + feed_rate) * b);
    
    // Mouse interaction: add chemical B at mouse position
    let mouse_pos = vec2<f32>(Mouse.position.x / resolution.x, 1.0 - Mouse.position.y / resolution.y);
    let mouse_distance = length(uv - mouse_pos);
    
    var final_a = clamp(new_a, 0.0, 1.0);
    var final_b = clamp(new_b, 0.0, 1.0);
    
    // Add chemical B where mouse is near
    if (mouse_distance < 0.05) {
        final_b = min(final_b + 0.5, 1.0);
        final_a = max(final_a - 0.1, 0.0);
    }
    
    // Initialize with some interesting patterns on first few seconds
    if (Time.duration < 0.1) {
        let center_dist = length(uv - vec2<f32>(0.5, 0.5));
        
        // Initialize mostly chemical A everywhere
        final_a = 1.0;
        final_b = 0.0;
        
        // Add some seed points of chemical B
        if (center_dist < 0.05) {
            final_b = 1.0;
            final_a = 0.0;
        }
        
        // Add some smaller seed points
        let seed1 = length(uv - vec2<f32>(0.3, 0.3));
        let seed2 = length(uv - vec2<f32>(0.7, 0.7));
        let seed3 = length(uv - vec2<f32>(0.2, 0.8));
        
        if (seed1 < 0.02 || seed2 < 0.02 || seed3 < 0.02) {
            final_b = 0.8;
            final_a = 0.2;
        }
    }
    
    // Create beautiful color mapping
    let concentration = final_b;
    let activity = final_a;
    
    // Color scheme based on chemical concentrations
    let red = concentration;
    let green = activity * concentration;
    let blue = activity * (1.0 - concentration);
    
    // Enhanced color mapping for better visualization
    let enhanced_red = pow(red, 0.8);
    let enhanced_green = pow(green, 1.2) * 0.7;
    let enhanced_blue = pow(blue, 0.6) * 0.9;
    
    // Add some ambient color to prevent pure black
    let ambient = 0.05;
    
    return vec4<f32>(
        enhanced_red + ambient,
        enhanced_green + ambient * 0.5,
        enhanced_blue + ambient * 0.3,
        1.0
    );
}