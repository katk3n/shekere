@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;

    // Sample previous frame content (this enables the trail effect)
    let previous = SamplePreviousPass(uv);

    // Fade the previous frame content (slower fade for longer trails)
    let fade_factor = 0.95;
    let faded_previous = previous * fade_factor;

    // Generate new content based on time and position
    let center = vec2<f32>(0.5, 0.5);
    let distance_from_center = length(uv - center);

    // Create a moving bright spot (larger and brighter for better trails)
    let time_factor = Time.duration * 1.5;

    // Spot1: Clockwise circular orbit
    let spot_position = center + vec2<f32>(
        cos(time_factor) * 0.3,
        sin(time_factor) * 0.3
    );
    let spot_distance = length(uv - spot_position);
    let spot_intensity = smoothstep(0.08, 0.02, spot_distance);

    // Spot2: Counter-clockwise elliptical orbit
    let spot2_position = center + vec2<f32>(
        cos(-time_factor * 0.7) * 0.25,
        sin(-time_factor * 0.7) * 0.4
    );
    let spot2_distance = length(uv - spot2_position);
    let spot2_intensity = smoothstep(0.06, 0.01, spot2_distance);

    // Create color variations (brighter spots for more visible trails)
    let new_content = vec4<f32>(
        spot_intensity * (0.7 + 0.3 * sin(time_factor * 0.5)),
        spot2_intensity * (0.7 + 0.3 * cos(time_factor * 0.7)),
        (spot_intensity + spot2_intensity) * (0.6 + 0.4 * sin(time_factor * 1.1)),
        1.0
    );

    // Add some background patterns that move slowly (reduced for better trail visibility)
    let pattern = sin(uv.x * 10.0 + time_factor * 0.1) * sin(uv.y * 10.0 + time_factor * 0.15);
    let background_glow = vec4<f32>(0.01, 0.005, 0.015, 1.0) * pattern * 0.05;

    // Combine faded previous frame with new content and background
    let result = faded_previous + new_content + background_glow;

    // Ensure we don't exceed 1.0 in any channel
    return vec4<f32>(
        min(result.r, 1.0),
        min(result.g, 1.0),
        min(result.b, 1.0),
        1.0
    );
}
