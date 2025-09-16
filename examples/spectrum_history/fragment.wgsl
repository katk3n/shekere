// Spectrum History Waterfall Visualization
// This example demonstrates spectrum history as a flowing waterfall display:
// - Horizontal axis shows time flowing from right (recent) to left (distant past)
// - Vertical axis shows frequency spectrum from low (bottom) to high (top)
// - Color intensity and hue represent amplitude levels
// - Creates professional audio analysis tool aesthetic

// Convert frequency index to a rainbow hue with emphasis on musical relevance
fn frequency_to_hue(freq_index: u32, total_freqs: u32, amplitude: f32) -> f32 {
    let freq_progress = f32(freq_index) / f32(total_freqs);

    // Create frequency-based color mapping:
    // Low frequencies (bass) = red/orange (0.0 - 0.15)
    // Mid frequencies = yellow/green (0.15 - 0.5)
    // High frequencies = blue/purple (0.5 - 0.8)
    let base_hue = freq_progress * 0.8;

    // Shift hue slightly based on amplitude for dynamic coloring
    let amplitude_shift = amplitude * 0.1;

    return (base_hue + amplitude_shift) % 1.0;
}

// Enhanced HSV to RGB with better saturation control
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let x = c * (1.0 - abs(((h * 6.0) % 2.0) - 1.0));
    let m = v - c;

    var rgb = vec3<f32>(0.0);
    let h_sector = u32(h * 6.0) % 6u;

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

// Create smooth amplitude-based brightness with logarithmic scaling
fn amplitude_to_brightness(amplitude: f32) -> f32 {
    if amplitude <= 0.0 {
        return 0.0;
    }

    // Logarithmic scaling for better visual dynamic range
    let log_amp = log(amplitude * 100.0 + 1.0) / log(101.0);
    return pow(log_amp, 0.7); // Gamma correction for better visual perception
}

// Add time-based color cycling for dynamic effects
fn time_color_modifier(time: f32, base_color: vec3<f32>) -> vec3<f32> {
    // Subtle breathing effect
    let breath = sin(time * 1.5) * 0.1 + 1.0;

    // Slow color temperature shift
    let temp_shift = sin(time * 0.3) * 0.05;
    let warm_color = base_color * vec3(1.0 + temp_shift, 1.0, 1.0 - temp_shift * 0.5);

    return warm_color * breath;
}

// Smooth interpolation between spectrum data points
fn interpolated_amplitude(freq_pos: f32, history: u32) -> f32 {
    let num_points = f32(SpectrumNumPoints());
    let exact_index = freq_pos * num_points;
    let lower_index = u32(floor(exact_index));
    let upper_index = min(lower_index + 1u, SpectrumNumPoints() - 1u);
    let fraction = exact_index - f32(lower_index);

    if lower_index >= SpectrumNumPoints() {
        return 0.0;
    }

    let lower_amp = SpectrumAmplitudeHistory(lower_index, history);
    let upper_amp = SpectrumAmplitudeHistory(upper_index, history);

    return mix(lower_amp, upper_amp, fraction);
}

// Main waterfall rendering function
fn draw_spectrum_waterfall(uv: vec2<f32>) -> vec3<f32> {
    // Map UV to time history (X) and frequency (Y)
    let time_pos = (1.0 - (uv.x + 1.0) * 0.5); // Convert from [-1,1] to [0,1], flip X so right = recent
    let freq_pos = (1.0 - (uv.y + 1.0) * 0.5); // Convert from [-1,1] to [0,1], flip Y so bottom = low freq, top = high freq

    // Map time position to history frame (0 = current, 511 = oldest)
    let history_depth = 200u; // Use 200 frames (~3.3 seconds at 60fps)
    let history_frame = u32(time_pos * f32(history_depth));

    if freq_pos < 0.0 || freq_pos > 1.0 || history_frame >= history_depth {
        return vec3<f32>(0.0);
    }

    // Get interpolated amplitude for smooth visualization
    let amplitude = interpolated_amplitude(freq_pos, history_frame);

    if amplitude <= 0.001 {
        return vec3<f32>(0.0);
    }

    // Create frequency-based hue
    let freq_index = u32(freq_pos * f32(SpectrumNumPoints()));
    let hue = frequency_to_hue(freq_index, SpectrumNumPoints(), amplitude);

    // Calculate brightness with time-based fadeout for waterfall effect
    let brightness = amplitude_to_brightness(amplitude);
    let age_factor = f32(history_frame) / f32(history_depth);
    let time_fade = 1.0 - age_factor * age_factor; // Quadratic fade for smoother transition

    // High saturation for vibrant colors, brightness controls intensity
    let saturation = 0.8 + amplitude * 0.2; // Higher amplitude = more saturated
    let final_brightness = brightness * time_fade * 2.0; // Boost overall brightness

    // Generate base color
    let base_color = hsv_to_rgb(hue, saturation, final_brightness);

    // Add time-based color modulation
    let final_color = time_color_modifier(Time.duration, base_color);

    return final_color;
}

// Add frequency grid lines for reference (now horizontal lines)
fn draw_frequency_grid(uv: vec2<f32>) -> f32 {
    let freq_pos = (1.0 - (uv.y + 1.0) * 0.5); // Y axis represents frequency, flipped so bottom = low freq

    // Draw lines at musical octave boundaries (approximate)
    // Use individual checks since WGSL requires constant array indexing
    var grid_intensity = 0.0;
    let line_width = 0.003;

    // Check each frequency line individually (now horizontal)
    let positions = array<f32, 8>(0.05, 0.12, 0.25, 0.40, 0.55, 0.70, 0.82, 0.92);

    // Unroll the loop to avoid variable array indexing
    var dist = abs(freq_pos - positions[0]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    dist = abs(freq_pos - positions[1]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    dist = abs(freq_pos - positions[2]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    dist = abs(freq_pos - positions[3]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    dist = abs(freq_pos - positions[4]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    dist = abs(freq_pos - positions[5]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    dist = abs(freq_pos - positions[6]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    dist = abs(freq_pos - positions[7]);
    if dist < line_width { grid_intensity = max(grid_intensity, (1.0 - dist / line_width) * 0.15); }

    return grid_intensity;
}

// Add subtle background gradient
fn background_gradient(uv: vec2<f32>) -> vec3<f32> {
    let time_pos = (1.0 - (uv.x + 1.0) * 0.5); // X axis now represents time

    // Horizontal gradient from dark blue (recent, right) to very dark (distant past, left)
    let right_color = vec3<f32>(0.02, 0.05, 0.12);    // Dark blue (recent)
    let left_color = vec3<f32>(0.005, 0.005, 0.02);   // Very dark (past)

    return mix(left_color, right_color, time_pos);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = NormalizedCoords(in.position.xy);

    // Start with background gradient
    var color = background_gradient(uv);

    // Add main spectrum waterfall
    color += draw_spectrum_waterfall(uv);

    // Add frequency grid lines
    let grid = draw_frequency_grid(uv);
    color += vec3<f32>(grid * 0.3, grid * 0.5, grid * 0.3); // Subtle green grid

    // Add subtle vignette effect
    let vignette = 1.0 - length(uv) * 0.2;
    color *= vignette;

    // Apply tone mapping to handle HDR colors gracefully
    color = color / (color + vec3<f32>(0.8));

    // Final gamma correction
    return vec4<f32>(ToLinearRgb(color), 1.0);
}