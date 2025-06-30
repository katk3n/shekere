# Shekere

<div align="center">
  <img src="shekere_logo.png" alt="Shekere Logo" width="320"/>
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
    let uv = normalized_coords(in.position.xy);
    let m = mouse_coords();
    
    // Time-based color animation
    let color = vec3(
        sin(time.duration + uv.x * 3.0) * 0.5 + 0.5,
        cos(time.duration + uv.y * 3.0 + m.x) * 0.5 + 0.5,
        sin(time.duration * 2.0 + length(uv) * 5.0) * 0.5 + 0.5
    );
    
    return vec4(to_linear_rgb(color), 1.0);
}
```

### Available Helper Functions

shekere provides these built-in helper functions:

#### Coordinate Helpers
```wgsl
// Convert screen position to normalized coordinates (-1.0 to 1.0)
fn normalized_coords(position: vec2<f32>) -> vec2<f32>

// Get normalized mouse coordinates
fn mouse_coords() -> vec2<f32>
```

#### Color Conversion
```wgsl
// Convert to linear RGB (gamma correction)
fn to_linear_rgb(col: vec3<f32>) -> vec3<f32>

// Convert to sRGB
fn to_srgb(col: vec3<f32>) -> vec3<f32>
```

#### MIDI Helpers (when MIDI is configured)
```wgsl
// Get MIDI note velocity (0.0-1.0) for note number (0-127)
fn midi_note(note_num: u32) -> f32

// Get MIDI control change value (0.0-1.0) for CC number (0-127)  
fn midi_control(cc_num: u32) -> f32
```

### Example Usage

#### Basic Animation
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = normalized_coords(in.position.xy);
    
    let color = vec3(
        sin(time.duration + uv.x) * 0.5 + 0.5,
        cos(time.duration + uv.y) * 0.5 + 0.5,
        sin(time.duration + length(uv)) * 0.5 + 0.5
    );
    
    return vec4(to_linear_rgb(color), 1.0);
}
```

#### Mouse Interaction
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = normalized_coords(in.position.xy);
    let m = mouse_coords();
    
    let dist = length(uv - m);
    let brightness = 1.0 - smoothstep(0.0, 0.5, dist);
    
    return vec4(vec3(brightness), 1.0);
}
```

#### MIDI Control
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = normalized_coords(in.position.xy);
    
    // Use MIDI note C4 (60) for color intensity
    let note_intensity = midi_note(60u);
    
    // Use MIDI CC 1 (modulation wheel) for animation speed
    let mod_wheel = midi_control(1u);
    let speed = 1.0 + mod_wheel * 5.0;
    
    let color = vec3(sin(time.duration * speed) * note_intensity);
    
    return vec4(to_linear_rgb(color), 1.0);
}
```

## Available Uniforms

All uniforms are automatically included and bound. You can directly use them in your shaders:

### Always Available Uniforms

#### WindowUniform - `window`
```wgsl
struct WindowUniform {
    resolution: vec2<f32>,  // [width, height] in pixels
}
// Usage: window.resolution.x, window.resolution.y
```

#### TimeUniform - `time`
```wgsl
struct TimeUniform {
    duration: f32,  // Seconds since program start
}
// Usage: time.duration
```

#### MouseUniform - `mouse`
```wgsl
struct MouseUniform {
    position: vec2<f32>,  // Mouse coordinates in pixels
}
// Usage: mouse.position.x, mouse.position.y
// Or use helper: mouse_coords() for normalized coordinates
```

### Sound Uniforms (when configured)

#### OscUniform - `osc` (when OSC is enabled)
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
// Usage: osc.trucks[0].gain, osc.trucks[1].note, etc.
```

#### SpectrumUniform - `spectrum` (when spectrum analysis is enabled)
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
// Usage: spectrum.data_points[i].amplitude, spectrum.max_amplitude, etc.
```

#### MidiUniform - `midi` (when MIDI is enabled)
```wgsl
struct MidiUniform {
    notes: array<vec4<f32>, 32>,      // Note velocities (packed)
    controls: array<vec4<f32>, 32>,   // Control change values (packed)
}
// Usage: Use helper functions midi_note() and midi_control() instead of direct access
```


## Sample Projects

The included examples directory contains the following samples:

- `examples/mouse/`: Mouse-controlled shader art
- `examples/spectrum/`: Audio spectrum analysis visualizer
- `examples/osc/`: TidalCycles integration shader art

Use these as reference to create your own shader art projects.

## Quick Reference

### Getting Started
1. Create a TOML config file with window settings and shader path
2. Write a fragment shader using the built-in uniforms and helpers
3. Run with `shekere config.toml`

### Essential Helpers
- `normalized_coords(position)` - Convert to -1.0 to 1.0 coordinates
- `mouse_coords()` - Get normalized mouse position
- `to_linear_rgb(color)` - Apply gamma correction
- `midi_note(60u)` - Get MIDI note velocity (when MIDI enabled)
- `midi_control(1u)` - Get MIDI CC value (when MIDI enabled)

### Essential Uniforms
- `window.resolution` - Screen size
- `time.duration` - Elapsed time
- `mouse.position` - Mouse position
- `osc.trucks[0].gain` - OSC data (when enabled)
- `spectrum.data_points[i].amplitude` - Audio data (when enabled)