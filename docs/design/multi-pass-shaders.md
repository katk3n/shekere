# Multi-Pass Shaders

## Overview

This document specifies the design for multi-pass shader functionality in shekere. The goal is to enable stateful shader art and post-effects while maintaining simplicity in configuration.

## Motivation

Multi-pass shaders enable two key capabilities:

1. **Stateful Shader Art**: Shaders that maintain state between frames (Game of Life, reaction-diffusion systems, particle systems)
2. **Post-Effects**: Multi-stage rendering pipelines for blur, color grading, and other effects

## Design Philosophy

**Convention over Configuration**: Minimize explicit configuration through reasonable defaults and automatic inference.

## Configuration Format

### Basic Multi-Pass (Post-Effects)

```toml
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Main Render"
entry_point = "fs_main"
file = "main.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Blur Effect"
entry_point = "fs_main"
file = "blur.wgsl"
```

**Automatic Inference**:
- First pass → intermediate texture `temp_0`
- Second pass → intermediate texture `temp_1` (if not final)
- Last pass → screen output
- Each pass automatically receives previous pass as input

### Ping-Pong Buffer (Stateful Shaders)

```toml
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Game of Life"
entry_point = "fs_main"
file = "life.wgsl"
ping_pong = true
```

**Automatic Inference**:
- Creates two textures: `buffer_a` and `buffer_b`
- Automatically alternates input/output between frames
- Automatically enables persistent state
- Final output to screen

### Persistent State (Accumulation Effects)

```toml
[window]
width = 800
height = 600

[[pipeline]]
shader_type = "fragment"
label = "Trail Effect"
entry_point = "fs_main"
file = "trail.wgsl"
persistent = true
```

**Behavior**:
- Texture content is preserved between frames
- Previous frame content is available as input
- Enables accumulation and trail effects

## Configuration Structure

### Extended ShaderConfig

```rust
#[derive(Debug, Deserialize, PartialEq)]
pub struct ShaderConfig {
    pub shader_type: String,
    pub label: String,
    pub entry_point: String,
    pub file: String,
    pub ping_pong: Option<bool>,    // Enable ping-pong buffer
    pub persistent: Option<bool>,   // Preserve state between frames
}
```

### Existing Config Structure (Unchanged)

```rust
#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    pub window: WindowConfig,
    pub pipeline: Vec<ShaderConfig>,
    pub osc: Option<OscConfig>,
    pub spectrum: Option<SpectrumConfig>,
    pub midi: Option<MidiConfig>,
    pub hot_reload: Option<HotReloadConfig>,
}
```

## Automatic Inference Rules

### Texture Management

- **Regular Multi-Pass**: Creates `temp_0`, `temp_1`, ... for intermediate results
- **Ping-Pong Buffer**: Creates `buffer_a`, `buffer_b` for alternating state
- **Final Pass**: Always outputs to screen
- **Texture Format**: Always `Rgba8Unorm`
- **Texture Size**: Always matches window size
- **Texture Filter**: Always `Linear`

### Binding Layout

- **Group 3**: Reserved for multi-pass textures
- **Binding 0**: Input texture from previous pass/frame
- **Binding 1**: Texture sampler (Linear filter)

### Rendering Order

- Passes execute in configuration array order
- Last pass always renders to screen
- Ping-pong buffers alternate automatically based on frame count

### State Management

- **Default**: Textures are cleared each frame
- **`persistent = true`**: Texture content preserved between frames
- **`ping_pong = true`**: Automatically enables persistent state

## Shader API

### Automatic Bindings

The system automatically generates these bindings for multi-pass shaders:

```wgsl
// Group 3: Multi-pass textures
@group(3) @binding(0) var previous_pass: texture_2d<f32>;  // or previous_frame for ping-pong
@group(3) @binding(1) var texture_sampler: sampler;
```

### Usage in Shaders

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    
    // Sample from previous pass/frame
    let previous = textureSample(previous_pass, texture_sampler, uv);
    
    // Apply effect
    let result = apply_effect(previous, uv);
    
    return result;
}
```

## Usage Examples

### Game of Life

```toml
[[pipeline]]
shader_type = "fragment"
label = "Game of Life"
entry_point = "fs_main"
file = "life.wgsl"
ping_pong = true
```

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    let cell_size = 1.0 / vec2<f32>(f32(window.width), f32(window.height));
    
    // Count neighbors from previous frame
    var neighbors = 0u;
    for (var i = -1; i <= 1; i++) {
        for (var j = -1; j <= 1; j++) {
            if (i == 0 && j == 0) { continue; }
            let neighbor_uv = uv + vec2<f32>(f32(i), f32(j)) * cell_size;
            let neighbor = textureSample(previous_frame, texture_sampler, neighbor_uv);
            if (neighbor.r > 0.5) { neighbors += 1u; }
        }
    }
    
    let current = textureSample(previous_frame, texture_sampler, uv);
    let alive = current.r > 0.5;
    
    // Apply Game of Life rules
    var new_state = false;
    if (alive && (neighbors == 2u || neighbors == 3u)) { new_state = true; }
    if (!alive && neighbors == 3u) { new_state = true; }
    
    return vec4<f32>(f32(new_state), 0.0, 0.0, 1.0);
}
```

### Reaction-Diffusion System

```toml
[[pipeline]]
shader_type = "fragment"
label = "Reaction Diffusion"
entry_point = "fs_main"
file = "reaction_diffusion.wgsl"
ping_pong = true
```

### Trail Effect

```toml
[[pipeline]]
shader_type = "fragment"
label = "Trail Effect"
entry_point = "fs_main"
file = "trail.wgsl"
persistent = true
```

```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    
    // Get previous frame content
    let previous = textureSample(previous_pass, texture_sampler, uv);
    
    // Generate new content (e.g., based on mouse position)
    let new_content = generate_new_content(uv);
    
    // Fade previous content and add new content
    let result = previous * 0.95 + new_content;
    
    return result;
}
```

### Multi-Stage Post-Effects

```toml
[[pipeline]]
shader_type = "fragment"
label = "Main Scene"
entry_point = "fs_main"
file = "scene.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Horizontal Blur"
entry_point = "fs_main"
file = "blur_horizontal.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Vertical Blur"
entry_point = "fs_main"
file = "blur_vertical.wgsl"

[[pipeline]]
shader_type = "fragment"
label = "Color Grading"
entry_point = "fs_main"
file = "color_grade.wgsl"
```

## Implementation Requirements

### Core Components to Modify

1. **`src/config.rs`**: Extend `ShaderConfig` with new optional fields
2. **`src/state.rs`**: Add multi-texture management and ping-pong state
3. **`src/pipeline.rs`**: Support multiple pipeline creation
4. **`src/bind_group_factory.rs`**: Add texture binding support
5. **Rendering Loop**: Implement multi-pass execution

### Render Target Management

- Create intermediate textures based on pipeline configuration
- Manage ping-pong buffer alternation
- Handle persistent state preservation

### Binding Group Extensions

- Add Group 3 for multi-pass textures
- Dynamic binding based on pass requirements
- Maintain compatibility with existing uniform system

## Backward Compatibility

Existing single-pass configurations work unchanged:

```toml
[[pipeline]]
shader_type = "fragment"
label = "Basic Shader"
entry_point = "fs_main"
file = "basic.wgsl"
# No ping_pong or persistent flags → single-pass behavior
```

## Performance Considerations

- Minimal memory overhead for unused features
- Efficient texture switching for ping-pong buffers
- Automatic cleanup of intermediate textures
- Frame rate should remain unaffected for single-pass shaders

## Future Extensions

While keeping the core simple, the design allows for future enhancements:

- Custom texture formats (if needed)
- Texture size scaling
- Conditional pass execution
- Multi-buffer ping-pong (more than 2 buffers)