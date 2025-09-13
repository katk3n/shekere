# Shader Development Guide

## Common Shader Structure
- **Always check `shaders/common.wgsl` first** - contains predefined structures and uniforms
- **Do NOT redefine** structures, uniforms, or functions that already exist in `common.wgsl`
- **Entry Point**: Fragment shaders use `fs_main` as the main function
- **Coordinate System**: Screen coordinates from 0.0 to 1.0

## Available Uniform Structures (from common.wgsl)

### WindowUniform
- `resolution: vec2<f32>` - Window size in physical pixels

### TimeUniform  
- `duration: f32` - Time elapsed since program started

### MouseUniform
- `position: vec2<f32>` - Mouse position in physical pixels

### SpectrumUniform
- `frequencies: array<vec4<f32>, 512>` - Frequency values (packed)
- `amplitudes: array<vec4<f32>, 512>` - Amplitude values (packed)
- `num_points: u32` - Number of data points
- `max_frequency: f32` - Frequency with max amplitude
- `max_amplitude: f32` - Maximum amplitude value

### OscUniform
- `sounds: array<vec4<i32>, 4>` - OSC sound values (packed)
- `ttls: array<vec4<f32>, 4>` - OSC time-to-live values
- `notes: array<vec4<f32>, 4>` - OSC note values
- `gains: array<vec4<f32>, 4>` - OSC gain values

### MidiUniform
- `notes: array<vec4<f32>, 32>` - Note velocities (128 values in 32 vec4s)

## Shader Configuration
- **Pipeline Definition**: Use `[[pipeline]]` sections in TOML
- **Multi-pass**: Chain multiple shaders with `sequential_passes`
- **Ping-pong Buffers**: Use for iterative algorithms
- **Persistent Textures**: Maintain state across frames

## Hot Reload Development
- **Live Editing**: Shaders automatically recompile on file changes
- **Error Recovery**: Application continues running with compilation errors
- **State Preservation**: Uniforms and textures maintained during reloads

## Shader File Organization
- **Fragment Shaders**: Typically named `fragment.wgsl`
- **Common Code**: Use `shaders/common.wgsl` for shared definitions
- **Example Structure**: Each example has its own directory with TOML + shader files