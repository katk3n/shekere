# Shekere

<div align="center">
  <img src="shekere_logo.png" alt="Shekere Logo" width="480"/>
</div>

shekere is a real-time shader art framework that combines WGSL shaders with sound input. It supports mouse interaction, OSC control (TidalCycles, etc.), and audio spectrum analysis.

## Installation

### Install from Cargo

```bash
cargo install shekere
```

### Download Binary

Download binaries from [Releases](https://github.com/katk3n/shekere/releases).

## Basic Usage

```bash
shekere <config_file>
```

### Examples:

```bash
# Run mouse-controlled shader art
shekere examples/mouse/mouse.toml

# Run audio spectrum analyzer visualizer
shekere examples/spectrum/spectrum.toml

# Run shader art with TidalCycles integration
shekere examples/osc/osc.toml

# Run MIDI-controlled shader art
shekere examples/midi/midi.toml
```

## Project Structure

A typical shekere project has the following structure:

```
my_project/
├── config.toml        # Configuration file
└── fragment.wgsl      # Fragment shader
```

## Configuration File (TOML)

### Basic Configuration

Minimum configuration required for all projects:

```toml
[window]
width = 800
height = 800

[[pipeline]]
shader_type = "fragment"
label = "Fragment Shader"
entry_point = "fs_main"
file = "fragment.wgsl"
```

### Optional Configuration

#### OSC (Integration with TidalCycles, etc.)

```toml
[osc]
port = 2020
addr_pattern = "/dirt/play"

[[osc.sound]]
name = "bd"    # Bass drum
id = 1

[[osc.sound]]
name = "sd"    # Snare drum
id = 2

[[osc.sound]]
name = "hc"    # Hi-hat
id = 3
```

#### Audio Spectrum Analysis

```toml
[spectrum]
min_frequency = 27.0
max_frequency = 2000.0
sampling_rate = 44100
```

#### MIDI Input

```toml
[midi]
enabled = true
```

Real-time MIDI input provides:
- 128 note velocities (0-127 mapped to 0.0-1.0)
- 128 control change values (0-127 mapped to 0.0-1.0)
- Access via `MidiNote(note_num)` and `MidiControl(cc_num)` helper functions

#### Hot Reload

Enable shader hot reload for live coding:

```toml
[hot_reload]
enabled = true
```

When enabled, the application automatically reloads the shader when the WGSL file is modified, allowing for real-time shader development without restarting the application.

## Shader Development Guide

### Basic Fragment Shader

shekere automatically includes common definitions (uniforms, bindings, and helper functions) in every shader. You only need to write your fragment shader logic:

```wgsl
// Common definitions are automatically included - no need to define uniforms!

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Use built-in helper functions
    let uv = NormalizedCoords(in.position.xy);
    let m = MouseCoords();
    
    // Time-based color animation
    let color = vec3(
        sin(Time.duration + uv.x * 3.0) * 0.5 + 0.5,
        cos(Time.duration + uv.y * 3.0 + m.x) * 0.5 + 0.5,
        sin(Time.duration * 2.0 + length(uv) * 5.0) * 0.5 + 0.5
    );
    
    return vec4(ToLinearRgb(color), 1.0);
}
```

### Available Helper Functions

shekere provides these built-in helper functions:

#### Coordinate Helpers
```wgsl
// Convert screen position to normalized coordinates (-1.0 to 1.0)
fn NormalizedCoords(position: vec2<f32>) -> vec2<f32>

// Get normalized mouse coordinates
fn MouseCoords() -> vec2<f32>
```

#### Color Conversion
```wgsl
// Convert to linear RGB (gamma correction)
fn ToLinearRgb(col: vec3<f32>) -> vec3<f32>

// Convert to sRGB
fn ToSrgb(col: vec3<f32>) -> vec3<f32>
```

#### MIDI Helpers (when MIDI is configured)
```wgsl
// Get MIDI note velocity (0.0-1.0) for note number (0-127)
fn MidiNote(note_num: u32) -> f32

// Get MIDI control change value (0.0-1.0) for CC number (0-127)  
fn MidiControl(cc_num: u32) -> f32
```

### Example Usage

#### Basic Animation
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = NormalizedCoords(in.position.xy);
    
    let color = vec3(
        sin(Time.duration + uv.x) * 0.5 + 0.5,
        cos(Time.duration + uv.y) * 0.5 + 0.5,
        sin(Time.duration + length(uv)) * 0.5 + 0.5
    );
    
    return vec4(ToLinearRgb(color), 1.0);
}
```

#### Mouse Interaction
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = NormalizedCoords(in.position.xy);
    let m = MouseCoords();
    
    let dist = length(uv - m);
    let brightness = 1.0 - smoothstep(0.0, 0.5, dist);
    
    return vec4(vec3(brightness), 1.0);
}
```

#### MIDI Control
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = NormalizedCoords(in.position.xy);
    
    // Use MIDI note C4 (60) for color intensity
    let note_intensity = MidiNote(60u);
    
    // Use MIDI CC 1 (modulation wheel) for animation speed
    let mod_wheel = MidiControl(1u);
    let speed = 1.0 + mod_wheel * 5.0;
    
    let color = vec3(sin(Time.duration * speed) * note_intensity);
    
    return vec4(ToLinearRgb(color), 1.0);
}
```

## Available Uniforms

All uniforms are automatically included and bound. You can directly use them in your shaders:

### Always Available Uniforms

#### WindowUniform - `Window`
```wgsl
struct WindowUniform {
    resolution: vec2<f32>,  // [width, height] in pixels
}
// Usage: Window.resolution.x, Window.resolution.y
```

#### TimeUniform - `Time`
```wgsl
struct TimeUniform {
    duration: f32,  // Seconds since program start
}
// Usage: Time.duration
```

#### MouseUniform - `Mouse`
```wgsl
struct MouseUniform {
    position: vec2<f32>,  // Mouse coordinates in pixels
}
// Usage: Mouse.position.x, Mouse.position.y
// Or use helper: MouseCoords() for normalized coordinates
```

### Sound Uniforms (when configured)

#### OscUniform - `Osc` (when OSC is enabled)
```wgsl
struct OscTruck {
    sound: i32,   // Sound ID (from config)
    ttl: f32,     // Time to live (duration)
    note: f32,    // Note/pitch value
    gain: f32,    // Volume/gain (0.0-1.0)
}

struct OscUniform {
    trucks: array<OscTruck, 16>,  // Corresponds to d1-d16 in TidalCycles
}
// Usage: Osc.trucks[0].gain, Osc.trucks[1].note, etc.
```

#### SpectrumUniform - `Spectrum` (when spectrum analysis is enabled)
```wgsl
struct SpectrumDataPoint {
    frequency: f32,
    amplitude: f32,
    _padding: vec2<u32>,
}

struct SpectrumUniform {
    data_points: array<SpectrumDataPoint, 2048>,
    num_points: u32,
    max_frequency: f32,
    max_amplitude: f32,
}
// Usage: Spectrum.data_points[i].amplitude, Spectrum.max_amplitude, etc.
```

#### MidiUniform - `Midi` (when MIDI is enabled)
```wgsl
struct MidiUniform {
    notes: array<vec4<f32>, 32>,      // Note velocities (packed)
    controls: array<vec4<f32>, 32>,   // Control change values (packed)
}
// Usage: Use helper functions MidiNote() and MidiControl() instead of direct access
```


## Sample Projects

The included examples directory contains the following samples:

- `examples/mouse/`: Mouse-controlled shader art
- `examples/spectrum/`: Audio spectrum analysis visualizer
- `examples/osc/`: TidalCycles integration shader art
- `examples/midi/`: MIDI-controlled shader art

Use these as reference to create your own shader art projects.

## Quick Reference

### Getting Started
1. Create a TOML config file with window settings and shader path
2. Write a fragment shader using the built-in uniforms and helpers
3. Run with `shekere config.toml`

### Essential Helpers
- `NormalizedCoords(position)` - Convert to -1.0 to 1.0 coordinates
- `MouseCoords()` - Get normalized mouse position
- `ToLinearRgb(color)` - Apply gamma correction
- `MidiNote(60u)` - Get MIDI note velocity (when MIDI enabled)
- `MidiControl(1u)` - Get MIDI CC value (when MIDI enabled)

### Essential Uniforms
- `Window.resolution` - Screen size
- `Time.duration` - Elapsed time
- `Mouse.position` - Mouse position
- `Osc.trucks[0].gain` - OSC data (when enabled)
- `Spectrum.data_points[i].amplitude` - Audio data (when enabled)