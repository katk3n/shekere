// MIDI History Piano Roll Example
// This example demonstrates MIDI history as a piano roll visualization:
// - Horizontal lines show note history over time
// - Left side = recent past, Right side = distant past

// More vibrant and saturated color function
fn note_to_color(note_index: u32, total_notes: u32) -> vec3<f32> {
    // Create multiple color cycling patterns for more variation
    let hue_base = f32(note_index) / f32(total_notes);

    // Multiple overlapping color cycles for richer palette
    let hue1 = (hue_base * 3.0) % 1.0; // Fast cycling
    let hue2 = (hue_base * 7.0) % 1.0; // Medium cycling
    let hue3 = (hue_base * 1.0) % 1.0; // Slow cycling

    // Convert each hue to highly saturated color
    let color1 = hsv_to_rgb(hue1, 1.0, 1.0);
    let color2 = hsv_to_rgb(hue2, 0.8, 1.0);
    let color3 = hsv_to_rgb(hue3, 0.9, 0.8);

    // Blend the colors for complexity
    return normalize(color1 * 0.5 + color2 * 0.3 + color3 * 0.2);
}

// High-saturation HSV to RGB conversion
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> vec3<f32> {
    let c = v * s;
    let x = c * (1.0 - abs((h * 6.0) % 2.0 - 1.0));
    let m = v - c;

    var rgb = vec3(0.0);

    if h < 0.166667 {        // Red to Yellow
        rgb = vec3(c, x, 0.0);
    } else if h < 0.333333 { // Yellow to Green
        rgb = vec3(x, c, 0.0);
    } else if h < 0.5 {      // Green to Cyan
        rgb = vec3(0.0, c, x);
    } else if h < 0.666667 { // Cyan to Blue
        rgb = vec3(0.0, x, c);
    } else if h < 0.833333 { // Blue to Magenta
        rgb = vec3(x, 0.0, c);
    } else {                 // Magenta to Red
        rgb = vec3(c, 0.0, x);
    }

    return rgb + vec3(m);
}

// Piano roll visualization function (vertical flow)
fn draw_piano_roll(uv: vec2<f32>) -> vec3<f32> {
    let note_count = 128u; // All MIDI notes 0-127
    let history_length = 120u; // Show 2 seconds of history at 60fps

    // Map UV coordinates to note and time (rotated 90 degrees)
    let note_index = u32((uv.x + 1.0) * 0.5 * f32(note_count)); // X axis = notes (left to right)
    let time_index = u32((1.0 - (uv.y + 1.0) * 0.5) * f32(history_length)); // Y axis = time (top to bottom)

    if note_index >= note_count || time_index >= history_length {
        return vec3(0.0);
    }

    let note_number = note_index; // Direct mapping: 0-127
    let history_frame = time_index;

    // Get historical MIDI data
    let note_velocity = MidiNoteHistory(note_number, history_frame);
    let note_attack = MidiNoteOnHistory(note_number, history_frame);

    var color = vec3(0.0);

    // Draw sustained notes as vibrant colored vertical lines
    if note_velocity > 0.0 {
        let note_color = note_to_color(note_index, note_count);
        let brightness = note_velocity;
        color += note_color * brightness * 3.0; // Brighter for more impact
    }

    // Draw note attacks as bright colorful flashes
    if note_attack > 0.0 {
        let attack_color = note_to_color(note_index, note_count);
        // Mix white with note color for bright flash
        color += mix(vec3(1.0), attack_color, 0.3) * note_attack * 5.0;
    }

    return color;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2(
        NormalizedCoords(in.position.xy).x,
        -NormalizedCoords(in.position.xy).y
    );

    var col = vec3(0.02); // Very dark background

    // Piano roll main area
    col += draw_piano_roll(uv);

    return vec4(ToLinearRgb(col), 1.0);
}