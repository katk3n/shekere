@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    let resolution = Window.resolution;
    let cell_size = 1.0 / resolution;
    
    // Mouse interaction: add cells where mouse is
    let mouse_pos = vec2<f32>(Mouse.position.x / resolution.x, 1.0 - Mouse.position.y / resolution.y);
    let mouse_distance = length(uv - mouse_pos);
    
    // Initialize with some patterns if this is early in the simulation
    var new_cell = false;
    
    // Add cells with mouse interaction (assume mouse is "pressed" when near)
    if (mouse_distance < 0.02) {
        new_cell = true;
    }
    
    // Get current cell state from previous frame
    let current_state = SamplePreviousPass(uv);
    let is_alive = current_state.r > 0.5;
    
    // Count neighbors from the previous frame
    var neighbors = 0u;
    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            if (dx == 0 && dy == 0) { continue; }
            
            let neighbor_uv = uv + vec2<f32>(f32(dx), f32(dy)) * cell_size;
            
            // Handle wrapping for toroidal topology
            let wrapped_uv = vec2<f32>(
                fract(neighbor_uv.x + 1.0),
                fract(neighbor_uv.y + 1.0)
            );
            
            let neighbor_state = SamplePreviousPass(wrapped_uv);
            if (neighbor_state.r > 0.5) {
                neighbors += 1u;
            }
        }
    }
    
    // Apply Conway's Game of Life rules
    var next_alive = false;
    
    if (is_alive) {
        // Living cell survives with 2 or 3 neighbors
        if (neighbors == 2u || neighbors == 3u) {
            next_alive = true;
        }
    } else {
        // Dead cell becomes alive with exactly 3 neighbors
        if (neighbors == 3u) {
            next_alive = true;
        }
    }
    
    // Override with new cells from mouse interaction
    if (new_cell) {
        next_alive = true;
    }
    
    // Add some initial pattern on the first few seconds
    if (Time.duration < 0.5) {
        // Create a glider pattern
        let center = vec2<f32>(0.5, 0.5);
        let glider_pos = (uv - center) * resolution;
        
        // Glider pattern (classic)
        if ((abs(glider_pos.x - 1.0) < 0.5 && abs(glider_pos.y - 0.0) < 0.5) ||
            (abs(glider_pos.x - 2.0) < 0.5 && abs(glider_pos.y - 1.0) < 0.5) ||
            (abs(glider_pos.x - 0.0) < 0.5 && abs(glider_pos.y - 2.0) < 0.5) ||
            (abs(glider_pos.x - 1.0) < 0.5 && abs(glider_pos.y - 2.0) < 0.5) ||
            (abs(glider_pos.x - 2.0) < 0.5 && abs(glider_pos.y - 2.0) < 0.5)) {
            next_alive = true;
        }
        
        // Add some random oscillators
        let blinker_pos = (uv - vec2<f32>(0.3, 0.3)) * resolution;
        if ((abs(blinker_pos.x - 0.0) < 0.5 && abs(blinker_pos.y - 0.0) < 0.5) ||
            (abs(blinker_pos.x - 1.0) < 0.5 && abs(blinker_pos.y - 0.0) < 0.5) ||
            (abs(blinker_pos.x - 2.0) < 0.5 && abs(blinker_pos.y - 0.0) < 0.5)) {
            next_alive = true;
        }
    }
    
    // Color scheme: living cells are white, dead cells fade to black
    let cell_color = select(0.0, 1.0, next_alive);
    
    // Add some visual effects for more interesting display
    let fade_factor = select(0.95, 1.0, next_alive); // Slight fade for dead cells
    let previous_intensity = current_state.r * fade_factor;
    let final_intensity = max(cell_color, previous_intensity * 0.1);
    
    // Color gradient based on neighbor count for visual interest
    let neighbor_intensity = f32(neighbors) / 8.0;
    
    return vec4<f32>(
        final_intensity,                           // Red: main cell state
        final_intensity * (1.0 - neighbor_intensity * 0.3), // Green: slightly affected by neighbors
        neighbor_intensity * 0.5,                 // Blue: neighbor count visualization
        1.0
    );
}