# Shekere Development Guide

This guide covers advanced usage patterns, shader development techniques, and integration details for shekere.

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

## Shader Development

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

## See Also

For complete reference documentation, see:

- **[API Reference](api-reference.md)** - Complete reference for all uniforms, structures, and helper functions

