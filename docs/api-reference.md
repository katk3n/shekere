# Shekere API Reference

This document provides a complete reference for all uniforms, structures, and helper functions available in shekere shaders.

## Vertex Output Structure

```wgsl
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
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

#### MidiUniform - `Midi`
```wgsl
struct MidiUniform {
    notes: array<vec4<f32>, 32>,      // Note velocities (packed)
    controls: array<vec4<f32>, 32>,   // Control change values (packed)
}
```
- **Usage**: Use helper functions `MidiNote()` and `MidiControl()` instead of direct access
- **Binding**: `@group(2) @binding(2)`
- **Note**: Values are normalized from 0-127 to 0.0-1.0

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

#### `MidiNote(note_num: u32) -> f32`
Gets MIDI note velocity for a specific note number.

- **Input**: MIDI note number (0-127)
- **Output**: Note velocity (0.0-1.0)
- **Range**: 0.0 = note off, 1.0 = maximum velocity
- **Invalid input**: Returns 0.0 for note numbers > 127

```wgsl
// Use MIDI note C4 (60) for color intensity
let note_intensity = MidiNote(60u);
let color = vec3(sin(Time.duration) * note_intensity);
```

#### `MidiControl(cc_num: u32) -> f32`
Gets MIDI control change value for a specific CC number.

- **Input**: MIDI CC number (0-127)
- **Output**: Control change value (0.0-1.0)
- **Range**: 0.0 = minimum, 1.0 = maximum
- **Invalid input**: Returns 0.0 for CC numbers > 127

```wgsl
// Use MIDI CC 1 (modulation wheel) for animation speed
let mod_wheel = MidiControl(1u);
let speed = 1.0 + mod_wheel * 5.0;
let color = vec3(sin(Time.duration * speed));
```

