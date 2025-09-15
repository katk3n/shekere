# Shekere Shader API Reference

Welcome to shekere! This reference covers all the functions and variables you can use to create interactive shader art that responds to time, mouse input, and audio data.

## Getting Started

Your fragment shader receives this input structure:

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Your creative code here
    let uv = NormalizedCoords(in.position.xy);
    return vec4(sin(Time.duration + uv.x), 0.0, 0.0, 1.0);
}
```

## Core Functions

### Time and Animation

#### `Time.duration`
Current time in seconds since the program started.

```wgsl
// Animate colors over time
let color = vec3(sin(Time.duration), cos(Time.duration), 0.5);

// Create pulsing effects
let pulse = 0.5 + 0.5 * sin(Time.duration * 2.0);
```

#### `Window.resolution`
Current window size as `vec2<f32>` (width, height) in pixels.

```wgsl
// Use window dimensions for effects
let aspect_ratio = Window.resolution.x / Window.resolution.y;
let screen_size = length(Window.resolution);

// Create patterns based on screen size
let grid_frequency = screen_size * 0.01;
```

### Coordinate System

#### `NormalizedCoords(position: vec2<f32>) -> vec2<f32>`
Converts screen pixels to normalized coordinates (-1.0 to 1.0), maintaining aspect ratio.

```wgsl
// Get normalized coordinates
let uv = NormalizedCoords(in.position.xy);

// Create centered patterns
let distance_from_center = length(uv);
let circle = smoothstep(0.5, 0.4, distance_from_center);
```

### Mouse Interaction

#### `MouseCoords() -> vec2<f32>`
Current mouse position in normalized coordinates.

```wgsl
// Mouse interaction
let uv = NormalizedCoords(in.position.xy);
let mouse = MouseCoords();
let dist_to_mouse = length(uv - mouse);

// Create mouse-following effects
let glow = exp(-dist_to_mouse * 10.0);
```

#### `MouseCoordsHistory(history: u32) -> vec2<f32>`
Mouse position from previous frames (0 = current, 511 = oldest).

```wgsl
// Mouse trail effect
var trail = vec3(0.0);
for (var i = 0u; i < 32u; i++) {
    let old_pos = MouseCoordsHistory(i * 4u);
    let dist = distance(uv, old_pos);
    let intensity = 1.0 - f32(i) / 32.0;
    trail += vec3(intensity) * exp(-dist * 20.0);
}
```

## Audio Functions

### Spectrum Analysis

#### `SpectrumFrequency(index: u32) -> f32`
Frequency value at spectrum index (0-2047).

#### `SpectrumAmplitude(index: u32) -> f32`
Amplitude value at spectrum index (0.0-1.0+).

```wgsl
// Audio-reactive bars
let bar_index = u32(uv.x * 64.0);  // 64 bars across screen
let amplitude = SpectrumAmplitude(bar_index * 32u);
let bar_height = amplitude * 2.0;

if (uv.y < bar_height && uv.y > 0.0) {
    color = vec3(amplitude, 0.5, 1.0 - amplitude);
}
```

### OSC Integration (TidalCycles)

#### Current Frame Functions
- `OscSound(index: u32) -> i32` - Sound ID for OSC track (0-15)
- `OscTtl(index: u32) -> f32` - Time remaining for OSC track
- `OscNote(index: u32) -> f32` - Note/pitch value for OSC track
- `OscGain(index: u32) -> f32` - Volume/gain for OSC track (0.0-1.0+)

#### History Functions (512 frames)
- `OscSoundHistory(index: u32, history: u32) -> i32` - Historical sound values
- `OscTtlHistory(index: u32, history: u32) -> f32` - Historical TTL values
- `OscNoteHistory(index: u32, history: u32) -> f32` - Historical note values
- `OscGainHistory(index: u32, history: u32) -> f32` - Historical gain values

Where `history`: 0 = current frame, 1 = 1 frame ago, ..., 511 = 511 frames ago

```wgsl
// React to TidalCycles patterns
if (OscSound(0u) == 1) {  // Check if kick drum is playing
    let gain = OscGain(0u);
    let flash = gain * exp(-OscTtl(0u) * 2.0);  // Fade based on time
    color += vec3(flash, 0.0, 0.0);
}

// OSC history effects
for (var i: u32 = 0u; i < 16u; i++) {
    let historical_gain = OscGainHistory(0u, i);
    let trail_position = coords + vec2(f32(i) * 0.01, 0.0);
    if (historical_gain > 0.1) {
        let trail_alpha = historical_gain * (1.0 - f32(i) / 16.0);
        color = mix(color, vec3(1.0, 0.5, 0.0), trail_alpha * 0.3);
    }
}
```

### MIDI Control

#### Current Frame Functions
- `MidiNote(note_num: u32) -> f32` - Note velocity (0-127 → 0.0-1.0)
- `MidiControl(cc_num: u32) -> f32` - Control change value
- `MidiNoteOn(note_num: u32) -> f32` - Attack detection (only non-zero on attack frame)

```wgsl
// MIDI keyboard interaction
let piano_c4 = MidiNote(60u);  // Middle C
let modwheel = MidiControl(1u);  // Modulation wheel

// Flash on note attacks
let kick_attack = MidiNoteOn(36u);
if (kick_attack > 0.0) {
    color += vec3(kick_attack * 2.0);  // White flash
}
```

#### History Functions
- `MidiNoteHistory(note_num: u32, history: u32) -> f32`
- `MidiControlHistory(cc_num: u32, history: u32) -> f32`
- `MidiNoteOnHistory(note_num: u32, history: u32) -> f32`

```wgsl
// MIDI echo effects
let kick_now = MidiNoteOnHistory(36u, 0u);
let echo1 = MidiNoteOnHistory(36u, 30u) * 0.5;  // 0.5s ago
let echo2 = MidiNoteOnHistory(36u, 60u) * 0.25; // 1.0s ago
let total_flash = kick_now + echo1 + echo2;
```

## Visual Effects Functions

### Color Conversion

#### `ToLinearRgb(col: vec3<f32>) -> vec3<f32>`
Converts sRGB to linear RGB (for proper color math).

#### `ToSrgb(col: vec3<f32>) -> vec3<f32>`
Converts linear RGB to sRGB (for display).

```wgsl
// Proper color blending
let linear_color = ToLinearRgb(base_color);
let blended = mix(linear_color, effect_color, 0.5);
return vec4(ToSrgb(blended), 1.0);
```

### Multi-Pass Effects

#### `SamplePreviousPass(uv: vec2<f32>) -> vec4<f32>`
Sample from previous rendering pass or frame.

#### `SamplePreviousPassOffset(uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32>`
Sample with UV offset (useful for blur/distortion).

```wgsl
// Feedback effect
let uv = in.tex_coords;
let previous = SamplePreviousPass(uv + vec2(0.01 * sin(Time.duration), 0.0));
let current = vec4(sin(Time.duration + uv.x), 0.0, 0.0, 1.0);
return mix(previous * 0.95, current, 0.1);  // Fade trail
```

```wgsl
// Blur effect
var blur = vec4(0.0);
let blur_amount = 0.01;
for (var x = -2; x <= 2; x++) {
    for (var y = -2; y <= 2; y++) {
        let offset = vec2(f32(x), f32(y)) * blur_amount;
        blur += SamplePreviousPassOffset(uv, offset);
    }
}
return blur / 25.0;  // Average of 5x5 samples
```

