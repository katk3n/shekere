struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
}

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
}

struct MouseUniform {
    // mouse position in physical size
    position: vec2<f32>,
}

struct MidiUniform {
    // note velocities (0-127 normalized to 0.0-1.0) 
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    notes: array<vec4<f32>, 32>,
    // control change values (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    cc: array<vec4<f32>, 32>,
}

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;
@group(1) @binding(0) var<uniform> mouse: MouseUniform;
@group(2) @binding(2) var<uniform> midi: MidiUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

fn to_linear_rgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return vec3(pow(c, vec3(gamma)));
}

// Helper function to get a note value from the packed array
fn get_note(note_num: u32) -> f32 {
    let vec4_index = note_num / 4u;
    let element_index = note_num % 4u;
    
    if vec4_index >= 32u {
        return 0.0;
    }
    
    let note_vec = midi.notes[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}

// Helper function to get a CC value from the packed array
fn get_cc(cc_num: u32) -> f32 {
    let vec4_index = cc_num / 4u;
    let element_index = cc_num % 4u;
    
    if vec4_index >= 32u {
        return 0.0;
    }
    
    let cc_vec = midi.cc[vec4_index];
    switch element_index {
        case 0u: { return cc_vec.x; }
        case 1u: { return cc_vec.y; }
        case 2u: { return cc_vec.z; }
        case 3u: { return cc_vec.w; }
        default: { return 0.0; }
    }
}

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
        let velocity = get_note(note_number);
        
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
    let min_xy = min(window.resolution.x, window.resolution.y);
    let uv = vec2(
        (in.position.x / min_xy) * 2.0 - 1.0,
        (1.0 - in.position.y / min_xy) * 2.0 - 1.0
    );
    
    let dist = length(uv);
    let angle = atan2(uv.y, uv.x);
    
    // Start with darker background but allow CC effects to modify it
    var col = vec3(0.05);
    
    // Background effects controlled by CCs (applied everywhere)
    
    // CC 1 (Modulation Wheel) - Controls circular wave patterns across entire screen
    let mod_wheel = get_cc(1u);
    if mod_wheel > 0.0 {
        let circle_intensity = sin(dist * 8.0 + time.duration * 2.0) * mod_wheel;
        col += hue_to_rgb(angle / 6.28318) * circle_intensity * 0.4;
    }
    
    // CC 10 (Pan) - Controls horizontal wave displacement across entire screen
    let pan = get_cc(10u);
    if pan > 0.0 {
        let pan_shift = (pan - 0.5) * 2.0; // Convert to -1.0 to 1.0
        let wave_x = sin((uv.x + pan_shift) * 10.0 + time.duration) * pan;
        col += vec3(wave_x * 0.3);
    }
    
    // CC 11 (Expression) - Controls vertical ripple effects across entire screen
    let expression = get_cc(11u);
    if expression > 0.0 {
        let ripple = sin(uv.y * 12.0 + time.duration * 3.0) * expression;
        col += vec3(ripple * 0.3);
    }
    
    // CC 20 - Controls spiral patterns across entire screen
    let cc20 = get_cc(20u);
    if cc20 > 0.0 {
        let spiral_angle = angle + dist * 4.0 * cc20 + time.duration;
        let spiral_intensity = sin(spiral_angle) * cc20;
        col += hue_to_rgb(spiral_angle / 6.28318) * spiral_intensity * 0.5;
    }
    
    // CC 21 - Controls radial pulsing from center across entire screen
    let cc21 = get_cc(21u);
    if cc21 > 0.0 {
        let pulse = sin(time.duration * 4.0 + dist * 3.0) * 0.5 + 0.5;
        let pulse_intensity = pulse * cc21;
        col += vec3(pulse_intensity * 0.4);
    }
    
    // CC 22 - Controls grid patterns across entire screen
    let cc22 = get_cc(22u);
    if cc22 > 0.0 {
        let grid_x = sin(uv.x * 20.0 * cc22 + time.duration);
        let grid_y = sin(uv.y * 20.0 * cc22 + time.duration);
        let grid_intensity = grid_x * grid_y * cc22;
        col += hue_to_rgb((uv.x + uv.y + time.duration) * 0.5) * grid_intensity * 0.5;
    }
    
    // Draw 4x4 drum machine pads (centered on screen)
    for (var i = 0u; i < 16u; i++) {
        let pad_col = draw_drum_pad(uv, i);
        if length(pad_col) > 0.0 {
            col = mix(col, pad_col, 0.9);
        }
    }
    
    // CC 7 (Volume) - Controls overall brightness (applied last)
    let volume = get_cc(7u);
    col *= (0.6 + volume * 0.8);
    
    return vec4(to_linear_rgb(col), 1.0);
}