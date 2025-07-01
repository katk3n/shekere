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
struct OscTruck {
    sound: i32,   // Sound ID (from configuration)
    ttl: f32,     // Time to live (duration of sound event)
    note: f32,    // Note/pitch value
    gain: f32,    // Volume/gain (0.0-1.0)
}

struct OscUniform {
    trucks: array<OscTruck, 16>,  // OSC trucks (d1-d16 in TidalCycles)
}
```
- **Usage**: `Osc.trucks[0].gain`, `Osc.trucks[1].note`, etc.
- **Binding**: `@group(2) @binding(0)`
- **Note**: `trucks[0]` corresponds to `d1` in TidalCycles, `trucks[1]` to `d2`, etc.

#### SpectrumUniform - `Spectrum`
```wgsl
struct SpectrumDataPoint {
    frequency: f32,
    amplitude: f32,
    _padding: vec2<u32>,
}

struct SpectrumUniform {
    data_points: array<SpectrumDataPoint, 2048>,  // Spectrum analysis data
    num_points: u32,                              // Number of valid data points
    max_frequency: f32,                           // Frequency with max amplitude
    max_amplitude: f32,                           // Maximum amplitude in current frame
}
```
- **Usage**: `Spectrum.data_points[i].amplitude`, `Spectrum.max_amplitude`, etc.
- **Binding**: `@group(2) @binding(1)`

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

