// Common definitions (including MidiUniform, bindings, and MIDI helpers) are automatically included

fn hue_to_rgb(hue: f32) -> vec3<f32> {
    let kr = (5.0 + hue * 6.0) % 6.0;
    let kg = (3.0 + hue * 6.0) % 6.0;
    let kb = (1.0 + hue * 6.0) % 6.0;

    let r = 1.0 - max(min(min(kr, 4.0 - kr), 1.0), 0.0);
    let g = 1.0 - max(min(min(kg, 4.0 - kg), 1.0), 0.0);
    let b = 1.0 - max(min(min(kb, 4.0 - kb), 1.0), 0.0);

    return vec3(r, g, b);
}

fn draw_drum_pad(uv: vec2<f32>, pad_index: u32) -> vec3<f32> {
    let row = pad_index / 4u; // 0-3
    let col = pad_index % 4u; // 0-3
    
    // Grid layout: 4x4 pads centered on screen
    let pad_size = 0.3; // Size of each pad (smaller for better fit)
    let pad_spacing = 0.4; // Spacing between pads (smaller for better fit)
    
    // Calculate grid dimensions
    let grid_width = 3.0 * pad_spacing; // 3 spaces between 4 pads
    let grid_height = 3.0 * pad_spacing; // 3 spaces between 4 rows
    
    // Center the entire grid on screen
    let start_x = -grid_width * 0.5;
    let start_y = -grid_height * 0.5;
    
    // Calculate individual pad position
    let pad_x = start_x + f32(col) * pad_spacing;
    let pad_y = start_y + f32(row) * pad_spacing;
    
    // Check if we're inside this pad
    let dx = abs(uv.x - pad_x);
    let dy = abs(uv.y - pad_y);
    
    if dx <= pad_size * 0.5 && dy <= pad_size * 0.5 {
        let note_number = 36u + pad_index; // Notes 36-51 (16 pads)
        let velocity = MidiNote(note_number);
        
        // Base pad color varies by position
        let base_hue = f32(pad_index) / 16.0;
        let base_color = hue_to_rgb(base_hue) * 0.4;
        
        // Add border effect for better visibility
        let border_factor = 1.0 - max(dx / (pad_size * 0.5), dy / (pad_size * 0.5));
        let border_intensity = smoothstep(0.7, 1.0, border_factor);
        
        if velocity > 0.0 {
            // Active pad: very bright color based on velocity with enhanced visibility
            let active_color = hue_to_rgb(base_hue) * velocity * 2.0;
            let flash_effect = vec3(velocity * 0.5); // White flash effect
            return base_color + active_color + flash_effect + vec3(border_intensity * 0.6);
        } else {
            // Inactive pad: brighter base color with more visible border
            return base_color + vec3(border_intensity * 0.2);
        }
    }
    
    return vec3(0.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2(
        NormalizedCoords(in.position.xy).x,
        -NormalizedCoords(in.position.xy).y
    );
    
    let dist = length(uv);
    let angle = atan2(uv.y, uv.x);
    
    // Start with darker background but allow CC effects to modify it
    var col = vec3(0.05);
    
    // Background effects controlled by CCs (applied everywhere)
    
    // CC 1 (Modulation Wheel) - Controls circular wave patterns across entire screen
    let mod_wheel = MidiControl(1u);
    if mod_wheel > 0.0 {
        let circle_intensity = sin(dist * 8.0 + Time.duration * 2.0) * mod_wheel;
        col += hue_to_rgb(angle / 6.28318) * circle_intensity * 0.4;
    }
    
    // CC 10 (Pan) - Controls horizontal wave displacement across entire screen
    let pan = MidiControl(10u);
    if pan > 0.0 {
        let pan_shift = (pan - 0.5) * 2.0; // Convert to -1.0 to 1.0
        let wave_x = sin((uv.x + pan_shift) * 10.0 + Time.duration) * pan;
        col += vec3(wave_x * 0.3);
    }
    
    // CC 11 (Expression) - Controls vertical ripple effects across entire screen
    let expression = MidiControl(11u);
    if expression > 0.0 {
        let ripple = sin(uv.y * 12.0 + Time.duration * 3.0) * expression;
        col += vec3(ripple * 0.3);
    }
    
    // CC 20 - Controls spiral patterns across entire screen
    let cc20 = MidiControl(20u);
    if cc20 > 0.0 {
        let spiral_angle = angle + dist * 4.0 * cc20 + Time.duration;
        let spiral_intensity = sin(spiral_angle) * cc20;
        col += hue_to_rgb(spiral_angle / 6.28318) * spiral_intensity * 0.5;
    }
    
    // CC 21 - Controls radial pulsing from center across entire screen
    let cc21 = MidiControl(21u);
    if cc21 > 0.0 {
        let pulse = sin(Time.duration * 4.0 + dist * 3.0) * 0.5 + 0.5;
        let pulse_intensity = pulse * cc21;
        col += vec3(pulse_intensity * 0.4);
    }
    
    // CC 22 - Controls grid patterns across entire screen
    let cc22 = MidiControl(22u);
    if cc22 > 0.0 {
        let grid_x = sin(uv.x * 20.0 * cc22 + Time.duration);
        let grid_y = sin(uv.y * 20.0 * cc22 + Time.duration);
        let grid_intensity = grid_x * grid_y * cc22;
        col += hue_to_rgb((uv.x + uv.y + Time.duration) * 0.5) * grid_intensity * 0.5;
    }
    
    // Draw 4x4 drum machine pads (centered on screen)
    for (var i = 0u; i < 16u; i++) {
        let pad_col = draw_drum_pad(uv, i);
        if length(pad_col) > 0.0 {
            col = mix(col, pad_col, 0.9);
        }
    }
    
    // CC 7 (Volume) - Controls overall brightness (applied last)
    let volume = MidiControl(7u);
    col *= (0.6 + volume * 0.8);
    
    return vec4(ToLinearRgb(col), 1.0);
}