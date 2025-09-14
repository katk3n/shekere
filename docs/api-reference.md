# Shekere API Reference

This document provides a complete reference for all uniforms, structures, and helper functions available in shekere shaders.

## Vertex Output Structure

```wgsl
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    tex_coords: vec2<f32>,
}
```

This structure is passed to your fragment shader's `fs_main` function:

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Your shader code here
}
```

## Binding Groups

Uniforms are organized into binding groups for efficient GPU access:

- **Group 0**: Always available (Window, Time)
- **Group 1**: Device uniforms (Mouse)
- **Group 2**: Sound uniforms (OSC, Spectrum, MIDI - only bound when configured)
- **Group 3**: Multi-pass textures (only available in multi-pass configurations)

## Uniform Structures

### Always Available Uniforms

#### WindowUniform - `Window`
```wgsl
struct WindowUniform {
    resolution: vec2<f32>,  // Window size in physical pixels [width, height]
}
```
- **Usage**: `Window.resolution.x`, `Window.resolution.y`
- **Binding**: `@group(0) @binding(0)`

#### TimeUniform - `Time`
```wgsl
struct TimeUniform {
    duration: f32,  // Time elapsed since program start (seconds)
}
```
- **Usage**: `Time.duration`
- **Binding**: `@group(0) @binding(1)`

### Device Uniforms (Group 1)

#### MouseUniform - `Mouse`
```wgsl
struct MouseUniform {
    position: vec2<f32>,  // Mouse position in physical pixels
}
```
- **Usage**: `Mouse.position.x`, `Mouse.position.y`
- **Binding**: `@group(1) @binding(0)`
- **Helper**: Use `MouseCoords()` for normalized coordinates

### Sound Uniforms (Group 2)

#### OscUniform - `Osc`
```wgsl
struct OscUniform {
    sounds: array<vec4<i32>, 4>,  // Sound IDs (packed into vec4s)
    ttls: array<vec4<f32>, 4>,    // Time to live values (packed)
    notes: array<vec4<f32>, 4>,   // Note/pitch values (packed)
    gains: array<vec4<f32>, 4>,   // Volume/gain values (packed)
}
```
- **Usage**: Use helper functions `OscSound()`, `OscTtl()`, `OscNote()`, `OscGain()` instead of direct access
- **Binding**: `@group(2) @binding(0)`
- **Note**: Index 0 corresponds to `d1` in TidalCycles, index 1 to `d2`, etc.

#### SpectrumUniform - `Spectrum`
```wgsl
struct SpectrumUniform {
    frequencies: array<vec4<f32>, 512>,  // Frequency values (packed into vec4s)
    amplitudes: array<vec4<f32>, 512>,   // Amplitude values (packed into vec4s)
    num_points: u32,                     // Number of valid data points
    max_frequency: f32,                  // Frequency with max amplitude
    max_amplitude: f32,                  // Maximum amplitude in current frame
}
```
- **Usage**: Use helper functions `SpectrumFrequency()`, `SpectrumAmplitude()` instead of direct access
- **Binding**: `@group(2) @binding(1)`
- **Note**: Total of 2048 data points (512 × 4) packed for WebGPU alignment

#### MidiHistory - `Midi`
```wgsl
struct MidiUniformData {
    notes: array<vec4<f32>, 32>,      // Note velocities (packed)
    controls: array<vec4<f32>, 32>,   // Control change values (packed)
    note_on: array<vec4<f32>, 32>,    // Note On attack detection (packed)
}

struct MidiHistory {
    history_data: array<MidiUniformData, 512>,  // 512 frames of MIDI history
}
```
- **Usage**: Use helper functions `MidiNote()`, `MidiControl()`, `MidiNoteOn()` for current frame, or `MidiNoteHistory()`, `MidiControlHistory()`, `MidiNoteOnHistory()` for historical data
- **Binding**: `@group(2) @binding(2)` (Storage Buffer)
- **Note**: Values are normalized from 0-127 to 0.0-1.0
- **History**: Index 0 = current frame, Index 511 = oldest frame (512 frames total)

### Multi-Pass Textures (Group 3)

#### Previous Pass/Frame Texture

```wgsl
@group(3) @binding(0) var previous_pass: texture_2d<f32>;
@group(3) @binding(1) var texture_sampler: sampler;
```

Multi-pass textures are automatically created and bound when using multi-pass shader configurations. The system creates different types of textures based on your configuration:

- **Multi-Pass Rendering**: `temp_0`, `temp_1`, etc. for intermediate render targets
- **Ping-Pong Buffers**: `buffer_a` and `buffer_b` that alternate each frame
- **Persistent Textures**: Single texture that preserves content between frames

**Automatic Texture Management:**
- First pass: No Group 3 binding (no previous pass exists)
- Subsequent passes: Previous pass output bound to Group 3
- Ping-pong mode: Previous frame state bound to Group 3
- Persistent mode: Previous frame content bound to Group 3

**Texture Properties:**
- **Format**: Matches screen format (typically `Bgra8UnormSrgb`)
- **Size**: Matches window dimensions
- **Filter**: Linear sampling
- **Usage**: Read-only in shaders (via `previous_pass` texture)

## Helper Functions

### Coordinate System

#### `NormalizedCoords(position: vec2<f32>) -> vec2<f32>`
Converts screen position to normalized coordinates (-1.0 to 1.0).

- **Input**: Screen position in pixels
- **Output**: Normalized coordinates where (-1,-1) is bottom-left, (1,1) is top-right
- **Aspect ratio**: Maintains aspect ratio using the smaller dimension

```wgsl
// Basic usage
let uv = NormalizedCoords(in.position.xy);

// Creating patterns based on position
let uv = NormalizedCoords(in.position.xy);
let color = vec3(sin(Time.duration + uv.x));
```

#### `MouseCoords() -> vec2<f32>`
Gets normalized mouse coordinates.

- **Output**: Normalized mouse coordinates (-1.0 to 1.0)
- **Equivalent to**: `NormalizedCoords(Mouse.position)`

```wgsl
// Mouse interaction
let uv = NormalizedCoords(in.position.xy);
let mouse = MouseCoords();
let dist = length(uv - mouse);
```

### Color Conversion

#### `ToLinearRgb(col: vec3<f32>) -> vec3<f32>`
Converts color to linear RGB space (applies gamma correction).

- **Input**: Color in sRGB space (0.0-1.0)
- **Output**: Color in linear RGB space
- **Gamma**: 2.2
- **Use case**: For correct color blending and lighting calculations

```wgsl
// Apply gamma correction for final output
let color = vec3(sin(Time.duration + uv.x));
return vec4(ToLinearRgb(color), 1.0);
```

#### `ToSrgb(col: vec3<f32>) -> vec3<f32>`
Converts color to sRGB space.

- **Input**: Color in linear RGB space
- **Output**: Color in sRGB space (0.0-1.0)
- **Gamma**: 1/2.2
- **Use case**: For display output (opposite of `ToLinearRgb`)

```wgsl
// Convert linear color back to sRGB
let linear_color = vec3(0.5, 0.3, 0.8);
let srgb_color = ToSrgb(linear_color);
```

### Multi-Pass Helpers

#### `SamplePreviousPass(uv: vec2<f32>) -> vec4<f32>`
Samples from the previous pass or frame texture.

- **Input**: UV coordinates (0.0-1.0)
- **Output**: Color from previous pass/frame
- **Availability**: Only in multi-pass configurations (not available in first pass)
- **Use case**: Accessing the result from the previous rendering pass or previous frame

```wgsl
// Basic multi-pass sampling
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    
    // Sample from previous pass
    let previous = SamplePreviousPass(uv);
    
    // Apply effect (e.g., blur, color correction)
    let result = apply_effect(previous, uv);
    
    return result;
}
```

#### `SamplePreviousPassOffset(uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32>`
Samples from the previous pass or frame texture with an offset.

- **Input**: UV coordinates (0.0-1.0) and offset in UV space
- **Output**: Color from previous pass/frame at offset position
- **Availability**: Only in multi-pass configurations (not available in first pass)
- **Use case**: Blur effects, neighborhood sampling, convolution filters

```wgsl
// Multi-sample blur effect
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    let blur_size = 0.01;
    
    var result = vec4(0.0);
    
    // Sample multiple points for blur
    for (var x = -2; x <= 2; x++) {
        for (var y = -2; y <= 2; y++) {
            let offset = vec2(f32(x), f32(y)) * blur_size;
            result += SamplePreviousPassOffset(uv, offset);
        }
    }
    
    return result / 25.0;  // Average of 5x5 samples
}
```

### Audio Spectrum Helpers

#### `SpectrumFrequency(index: u32) -> f32`
Gets frequency value at a specific spectrum index.

- **Input**: Spectrum data index (0-2047)
- **Output**: Frequency value in Hz
- **Range**: Depends on configuration (min_frequency to max_frequency)
- **Invalid input**: Returns 0.0 for index > 2047

```wgsl
// Use spectrum data for color visualization
for (var i = 0u; i < Spectrum.num_points; i++) {
    let freq = SpectrumFrequency(i);
    let amp = SpectrumAmplitude(i);
    // Create color based on frequency and amplitude
}
```

#### `SpectrumAmplitude(index: u32) -> f32`
Gets amplitude value at a specific spectrum index.

- **Input**: Spectrum data index (0-2047)
- **Output**: Amplitude value (0.0-1.0+)
- **Range**: 0.0 = silence, 1.0+ = loud (can exceed 1.0)
- **Invalid input**: Returns 0.0 for index > 2047

```wgsl
// Create bars visualization
let height = SpectrumAmplitude(i);
if (uv.y < height) {
    color = vec3(1.0, 0.0, 0.0);  // Red for audio activity
}
```

### OSC Helpers

#### `OscSound(index: u32) -> i32`
Gets sound ID for a specific OSC track.

- **Input**: OSC track index (0-15)
- **Output**: Sound ID (from configuration)
- **Range**: Depends on sound mapping in config
- **Invalid input**: Returns 0 for index > 15

```wgsl
// Check if kick drum (sound ID 1) is playing on track 0
if (OscSound(0u) == 1) {
    gain = OscGain(0u);  // Get the gain for this sound
}
```

#### `OscTtl(index: u32) -> f32`
Gets time-to-live for a specific OSC track.

- **Input**: OSC track index (0-15)
- **Output**: Time remaining (seconds)
- **Range**: 0.0+ (decreases over time)
- **Invalid input**: Returns 0.0 for index > 15

```wgsl
// Fade effect based on time remaining
let fade = OscTtl(0u) * 0.1;  // Scale factor
let color = vec3(fade);
```

#### `OscNote(index: u32) -> f32`
Gets note/pitch value for a specific OSC track.

- **Input**: OSC track index (0-15)
- **Output**: Note value (often MIDI note number)
- **Range**: Depends on OSC message content
- **Invalid input**: Returns 0.0 for index > 15

```wgsl
// Use note value for frequency-based effects
let note = OscNote(0u);
let freq = 440.0 * pow(2.0, (note - 69.0) / 12.0);  // Convert MIDI to Hz
```

#### `OscGain(index: u32) -> f32`
Gets gain/volume for a specific OSC track.

- **Input**: OSC track index (0-15)
- **Output**: Gain value (0.0-1.0+)
- **Range**: 0.0 = silent, 1.0+ = loud
- **Invalid input**: Returns 0.0 for index > 15

```wgsl
// Use gain for brightness control
let brightness = OscGain(0u);
let color = vec3(brightness);
```

### MIDI Helpers

The MIDI system provides both simple current-frame functions and powerful history-aware functions for advanced audio-visual programming.

#### Current Frame Functions (Backward Compatible)

##### `MidiNote(note_num: u32) -> f32`
Gets current MIDI note velocity for a specific note number.

- **Input**: MIDI note number (0-127)
- **Output**: Note velocity (0.0-1.0)
- **Range**: 0.0 = note off, 1.0 = maximum velocity
- **Invalid input**: Returns 0.0 for note numbers > 127
- **Implementation**: Wrapper for `MidiNoteHistory(note_num, 0u)`

```wgsl
// Simple current note access (unchanged from previous versions)
let piano_c4 = MidiNote(60u);  // Middle C
let color = vec3(piano_c4);
```

##### `MidiControl(cc_num: u32) -> f32`
Gets current MIDI control change value for a specific CC number.

- **Input**: MIDI CC number (0-127)
- **Output**: Control change value (0.0-1.0)
- **Range**: 0.0 = minimum, 1.0 = maximum
- **Invalid input**: Returns 0.0 for CC numbers > 127
- **Implementation**: Wrapper for `MidiControlHistory(cc_num, 0u)`

```wgsl
// Simple current control access
let modwheel = MidiControl(1u);  // Modulation wheel
let speed = 1.0 + modwheel * 3.0;
```

##### `MidiNoteOn(note_num: u32) -> f32`
Gets current MIDI Note On attack detection for a specific note number.

- **Input**: MIDI note number (0-127)
- **Output**: Note attack velocity (0.0-1.0)
- **Range**: 0.0 = no attack, 1.0 = maximum attack velocity
- **Duration**: Only non-zero for the exact frame when Note On occurs
- **Invalid input**: Returns 0.0 for note numbers > 127
- **Implementation**: Wrapper for `MidiNoteOnHistory(note_num, 0u)`

```wgsl
// Simple current attack detection
let kick_attack = MidiNoteOn(36u);  // Kick drum
if kick_attack > 0.0 {
    // Flash on attack
    color = vec3(kick_attack * 2.0);
}
```

#### History-Aware Functions (New in v0.11.0)

##### `MidiNoteHistory(note_num: u32, history: u32) -> f32`
Gets MIDI note velocity for a specific note number at a specific point in history.

- **Input**:
  - `note_num`: MIDI note number (0-127)
  - `history`: Frame history (0-511, where 0 = current frame, 511 = oldest frame)
- **Output**: Note velocity (0.0-1.0)
- **Range**: 0.0 = note off, 1.0 = maximum velocity
- **Invalid input**: Returns 0.0 for note numbers > 127 or history > 511

```wgsl
// Create fade trail using historical data
let current = MidiNoteHistory(60u, 0u);    // Current frame
let fade1 = MidiNoteHistory(60u, 10u) * 0.8;  // 10 frames ago
let fade2 = MidiNoteHistory(60u, 20u) * 0.6;  // 20 frames ago
let combined = max(current, max(fade1, fade2));

// Smooth parameter changes using historical averaging
var smooth_note = 0.0;
for (var i = 0u; i < 10u; i++) {
    smooth_note += MidiNoteHistory(60u, i);
}
smooth_note /= 10.0;  // Average over last 10 frames
```

##### `MidiControlHistory(cc_num: u32, history: u32) -> f32`
Gets MIDI control change value for a specific CC number at a specific point in history.

- **Input**:
  - `cc_num`: MIDI CC number (0-127)
  - `history`: Frame history (0-511, where 0 = current frame, 511 = oldest frame)
- **Output**: Control change value (0.0-1.0)
- **Range**: 0.0 = minimum, 1.0 = maximum
- **Invalid input**: Returns 0.0 for CC numbers > 127 or history > 511

```wgsl
// Analyze control parameter trends
let current_mod = MidiControlHistory(1u, 0u);
let previous_mod = MidiControlHistory(1u, 30u);  // 0.5s ago at 60fps
let delta = current_mod - previous_mod;  // Rate of change

// Use trend for dynamic effects
if delta > 0.1 {
    color += vec3(0.2, 0.0, 0.0);  // Red flash when increasing rapidly
}
```

##### `MidiNoteOnHistory(note_num: u32, history: u32) -> f32`
Gets MIDI Note On attack detection for a specific note number at a specific point in history.

- **Input**:
  - `note_num`: MIDI note number (0-127)
  - `history`: Frame history (0-511, where 0 = current frame, 511 = oldest frame)
- **Output**: Note attack velocity (0.0-1.0)
- **Range**: 0.0 = no attack, 1.0 = maximum attack velocity
- **Duration**: Only non-zero for the exact frame when Note On occurs
- **Invalid input**: Returns 0.0 for note numbers > 127 or history > 511

```wgsl
// Create echo effects using historical attacks
let kick_now = MidiNoteOnHistory(36u, 0u);   // Current attack
let echo1 = MidiNoteOnHistory(36u, 30u) * 0.5;  // 0.5s ago
let echo2 = MidiNoteOnHistory(36u, 60u) * 0.25; // 1.0s ago

var flash_intensity = kick_now;
if echo1 > 0.0 { flash_intensity += echo1; }
if echo2 > 0.0 { flash_intensity += echo2; }

// Analyze rhythmic patterns over time
var rhythm_density = 0.0;
for (var i = 0u; i < 240u; i++) {  // Last 4 seconds at 60fps
    if MidiNoteOnHistory(36u, i) > 0.0 {
        rhythm_density += 1.0;
    }
}
rhythm_density /= 240.0;  // Attacks per frame (0.0-1.0)

// Use for adaptive visual complexity
let complexity_factor = rhythm_density * 5.0;
```

#### Advanced History Patterns

```wgsl
// Polyrhythmic visualization using multiple instruments
let kick_pattern = MidiNoteOnHistory(36u, frame_offset);    // Kick
let snare_pattern = MidiNoteOnHistory(38u, frame_offset);   // Snare
let hihat_pattern = MidiNoteOnHistory(42u, frame_offset);   // Hi-hat

// Create visual layers based on instrument combinations
if kick_pattern > 0.0 && snare_pattern > 0.0 {
    // Special effect for simultaneous hits
    color += vec3(1.0, 1.0, 0.0) * (kick_pattern + snare_pattern);
}

// Time-stretched audio analysis
var bass_energy = 0.0;
for (var i = 0u; i < 60u; i++) {  // Last 1 second
    // Check multiple bass notes
    bass_energy += MidiNoteHistory(36u, i);  // Kick
    bass_energy += MidiNoteHistory(41u, i);  // Low tom
}
let bass_intensity = bass_energy / 60.0;

// Use for low-frequency visual effects
let bass_radius = bass_intensity * 2.0;
let bass_circle = length(uv) < bass_radius ? 1.0 : 0.0;
```

