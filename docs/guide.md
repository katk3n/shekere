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

#### OSC Integration

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

#### Input History

All input sources (Mouse, OSC, MIDI, Spectrum) provide 512 frames of history for trail effects and temporal analysis. See [API Reference](api-reference.md) for details.

#### Hot Reload

Enable shader hot reload for live coding:

```toml
[hot_reload]
enabled = true
```

When enabled, the application automatically reloads the shader when the WGSL file is modified, allowing for real-time shader development without restarting the application.

## Multi-Pass Shaders

Multi-pass shaders enable advanced rendering techniques by executing multiple shader passes in sequence. Each pass can access the output of the previous pass, enabling post-effects, state preservation, and complex visual systems.

### Multi-Pass Rendering

Execute multiple shaders in sequence, with each shader processing the output of the previous one:

```toml
[window]
width = 800
height = 600

# First pass: Generate scene
[[pipeline]]
shader_type = "fragment"
label = "Main Scene"
entry_point = "fs_main"
file = "scene.wgsl"

# Second pass: Apply blur effect
[[pipeline]]
shader_type = "fragment"
label = "Blur Effect"
entry_point = "fs_main"
file = "blur.wgsl"

# Third pass: Color grading (optional)
[[pipeline]]
shader_type = "fragment"
label = "Color Grading"
entry_point = "fs_main"
file = "color_grade.wgsl"
```

**Automatic behavior:**
- First pass renders to intermediate texture `temp_0`
- Second pass reads from `temp_0`, renders to `temp_1`
- Final pass always renders to screen
- Each pass automatically receives previous pass output via Group 3 bindings

### Ping-Pong Buffers

Use double-buffering for stateful shaders like cellular automata or reaction-diffusion systems:

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

**Automatic behavior:**
- Creates two textures: `buffer_a` and `buffer_b`
- Alternates reading/writing between buffers each frame
- Current frame reads from one buffer, writes to the other
- Enables stateful shader effects that evolve over time

### Persistent Textures

Preserve texture content between frames for accumulation effects:

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

**Automatic behavior:**
- Single texture preserves content between frames
- Previous frame content available via Group 3 bindings
- Enables trail effects, accumulation, and feedback systems
- First frame starts with cleared texture

## Shader Development

### Basic Fragment Shader

shekere automatically includes common definitions (uniforms, bindings, and helper functions) in every shader. You only need to write your fragment shader logic:

```wgsl
// Common definitions are automatically included - no need to define uniforms!

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

### Example Usage

For practical examples, see the `examples/` directory:

- **[examples/basic/](../examples/basic/)** - Basic fragment shader with time-based animation
- **[examples/circular/](../examples/circular/)** - Circular patterns and concentric rings
- **[examples/mouse/](../examples/mouse/)** - Mouse interaction and cursor-based effects
- **[examples/midi/](../examples/midi/)** - MIDI control integration
- **[examples/osc/](../examples/osc/)** - OSC integration
- **[examples/spectrum/](../examples/spectrum/)** - Audio spectrum visualization

### Multi-Pass Shader Examples

For practical multi-pass examples, see the `examples/` directory:

- **[examples/multi_pass/](../examples/multi_pass/)** - Basic blur effect (scene → blur)
- **[examples/persistent/](../examples/persistent/)** - Trail effects using persistent textures
- **[examples/ping_pong/](../examples/ping_pong/)** - Ping-pong buffer examples (kaleidoscope feedback)

#### Usage Patterns

**When to use Multi-Pass Rendering:**
- Post-processing effects (blur, bloom, tone mapping)
- Complex lighting calculations
- Multi-stage image filters
- Composition of multiple render passes

**When to use Ping-Pong Buffers:**
- Cellular automata (Game of Life, Langton's Ant)
- Fluid simulations
- Reaction-diffusion systems
- Any algorithm requiring current state based on previous state

**When to use Persistent Textures:**
- Trail and accumulation effects
- Paint/drawing applications
- Feedback systems
- Long-term state preservation

**Performance Considerations:**
- Multi-pass rendering uses additional GPU memory for intermediate textures
- Ping-pong buffers double the texture memory usage
- Complex multi-pass chains may impact frame rate

## See Also

For complete reference documentation, see:

- **[API Reference](api-reference.md)** - Complete reference for all uniforms, structures, and helper functions

