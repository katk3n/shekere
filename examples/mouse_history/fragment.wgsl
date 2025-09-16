// Mouse History Trail Example
// This example demonstrates mouse history as a beautiful trailing effect:
// - Rainbow-colored trails follow mouse movement
// - Multiple trail visualizations: dots, lines, and glow effects
// - Older positions fade out gradually creating smooth motion trails

// Generate rainbow colors based on position in trail
fn trail_to_color(trail_index: u32, total_trails: u32) -> vec3<f32> {
    let progress = f32(trail_index) / f32(total_trails);

    // Create rainbow hue cycling through spectrum
    let hue = progress * 6.0; // 6.0 for full spectrum

    // High saturation and brightness for vibrant colors
    return hsv_to_rgb(hue, 0.8, 1.0);
}

// HSV to RGB conversion for rainbow effects
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let x = c * (1.0 - abs((h % 2.0) - 1.0));
    let m = v - c;

    var rgb = vec3<f32>(0.0);
    let h_sector = u32(h) % 6u;

    switch h_sector {
        case 0u: { rgb = vec3(c, x, 0.0); }
        case 1u: { rgb = vec3(x, c, 0.0); }
        case 2u: { rgb = vec3(0.0, c, x); }
        case 3u: { rgb = vec3(0.0, x, c); }
        case 4u: { rgb = vec3(x, 0.0, c); }
        case 5u: { rgb = vec3(c, 0.0, x); }
        default: { rgb = vec3(c, x, 0.0); }
    }

    return rgb + vec3(m);
}

// Smooth distance function for circular trails
fn smooth_circle(uv: vec2<f32>, center: vec2<f32>, radius: f32, smoothness: f32) -> f32 {
    let dist = distance(uv, center);
    return 1.0 - smoothstep(radius - smoothness, radius + smoothness, dist);
}

// Create glowing trail effect
fn glow_effect(uv: vec2<f32>, center: vec2<f32>, intensity: f32) -> f32 {
    let dist = distance(uv, center);
    // Multiple glow layers for richer effect
    let glow1 = exp(-dist * 8.0) * intensity;
    let glow2 = exp(-dist * 3.0) * intensity * 0.3;
    let glow3 = exp(-dist * 1.0) * intensity * 0.1;
    return glow1 + glow2 + glow3;
}

// Draw connected line between two points
fn draw_line(uv: vec2<f32>, start: vec2<f32>, end: vec2<f32>, thickness: f32) -> f32 {
    let line_vec = end - start;
    let uv_vec = uv - start;

    // Project uv onto line
    let line_length_sq = dot(line_vec, line_vec);
    if line_length_sq < 0.001 {
        return 0.0;
    }

    let t = clamp(dot(uv_vec, line_vec) / line_length_sq, 0.0, 1.0);
    let closest_point = start + t * line_vec;
    let dist = distance(uv, closest_point);

    return 1.0 - smoothstep(thickness * 0.5, thickness, dist);
}

// Main trail rendering function
fn draw_mouse_trails(uv: vec2<f32>) -> vec3<f32> {
    var color = vec3<f32>(0.0);

    // Configuration
    let max_history = 128u;  // Show 128 frames of history (about 2 seconds at 60fps)
    let dot_base_size = 0.02;
    let line_thickness = 0.01;

    // Current mouse position (brightest)
    let current_pos = MouseCoordsHistory(0u);
    let current_color = vec3<f32>(1.0, 1.0, 1.0); // White for current

    // Draw current position as bright white dot
    color += current_color * smooth_circle(uv, current_pos, dot_base_size * 2.0, 0.01);
    color += current_color * glow_effect(uv, current_pos, 0.8);

    // Draw trail history
    for (var i = 1u; i < max_history; i += 1u) {
        let history_pos = MouseCoordsHistory(i);
        let age_factor = f32(i) / f32(max_history);
        let intensity = (1.0 - age_factor) * (1.0 - age_factor); // Quadratic fade

        if intensity < 0.01 {
            continue; // Skip very faded trails for performance
        }

        // Rainbow color based on position in trail
        let trail_color = trail_to_color(i, max_history);

        // Draw dots for each historical position
        let dot_size = dot_base_size * (1.0 - age_factor * 0.7); // Shrink with age
        let dot_contribution = smooth_circle(uv, history_pos, dot_size, 0.005) * intensity;
        color += trail_color * dot_contribution * 2.0;

        // Draw glow around each dot
        let glow_contribution = glow_effect(uv, history_pos, intensity * 0.3);
        color += trail_color * glow_contribution;

        // Draw connecting lines between consecutive positions
        if i > 1u {
            let prev_pos = MouseCoordsHistory(i - 1u);
            let line_contribution = draw_line(uv, prev_pos, history_pos, line_thickness * intensity);
            color += trail_color * line_contribution * intensity;
        }
    }

    return color;
}

// Add some background pattern for visual interest
fn background_pattern(uv: vec2<f32>) -> vec3<f32> {
    // Subtle grid pattern
    let grid_size = 20.0;
    let grid_uv = uv * grid_size;
    let grid_lines = abs(fract(grid_uv) - 0.5) * 2.0;
    let grid_factor = min(grid_lines.x, grid_lines.y);
    let grid_intensity = smoothstep(0.9, 1.0, grid_factor) * 0.02;

    // Animated radial gradient from center
    let center_dist = length(uv);
    let radial_gradient = sin(center_dist * 10.0 - Time.duration * 2.0) * 0.01;

    return vec3<f32>(grid_intensity + max(radial_gradient, 0.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = NormalizedCoords(in.position.xy);

    // Start with animated background
    var color = background_pattern(uv);

    // Add mouse trails (main effect)
    color += draw_mouse_trails(uv);

    // Add subtle vignette effect
    let vignette = 1.0 - length(uv) * 0.3;
    color *= vignette;

    // Apply tone mapping for HDR-like effect
    color = color / (color + vec3<f32>(1.0));

    return vec4<f32>(ToLinearRgb(color), 1.0);
}