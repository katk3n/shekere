# KaCHoFuGeTsu

kchfgt ("花鳥風月", which means beauties of nature) is a real-time shader art framework that combines WGSL shaders with sound input. It supports mouse interaction, OSC control (TidalCycles, etc.), and audio spectrum analysis.

## Installation

### Install from Cargo

```bash
cargo install kchfgt
```

### Download Binary

Download binaries from [Releases](https://github.com/katk3n/kchfgt/releases).

## Basic Usage

```bash
kchfgt <config_file>
```

### Examples:

```bash
# Run mouse-controlled shader art
kchfgt examples/mouse/mouse.toml

# Run audio spectrum analyzer visualizer
kchfgt examples/spectrum/spectrum.toml

# Run shader art with TidalCycles integration
kchfgt examples/osc/osc.toml
```

## Project Structure

A typical kchfgt project has the following structure:

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

```wgsl
// Required uniform structures
struct WindowUniform {
    resolution: vec2<f32>,
}

struct TimeUniform {
    duration: f32,
}

// Required uniforms
@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate normalized UV coordinates
    let min_xy = min(window.resolution.x, window.resolution.y);
    let uv = (in.position.xy * 2.0 - window.resolution) / min_xy;
    
    // Time-based color animation
    let color = vec3(
        sin(time.duration) * 0.5 + 0.5,
        cos(time.duration) * 0.5 + 0.5,
        sin(time.duration * 2.0) * 0.5 + 0.5
    );
    
    return vec4(color, 1.0);
}
```

## Available Uniforms

### 1. WindowUniform (Required)
- **Binding**: `@group(0) @binding(0)`
- **Content**: Window resolution information

```wgsl
struct WindowUniform {
    resolution: vec2<f32>,  // [width, height]
}
```

### 2. TimeUniform (Required)
- **Binding**: `@group(0) @binding(1)`
- **Content**: Elapsed time

```wgsl
struct TimeUniform {
    duration: f32,  // Seconds since program start
}
```

### 3. MouseUniform (Optional)
- **Binding**: `@group(1) @binding(0)`
- **Content**: Mouse position

```wgsl
struct MouseUniform {
    position: vec2<f32>,  // Mouse coordinates in pixels
}
```

### 4. OscUniform (When OSC is configured)
- **Binding**: `@group(2) @binding(0)`
- **Content**: OSC parameters (TidalCycles, etc.)

```wgsl
struct OscTruck {
    sound: i32,   // Sound ID
    ttl: f32,     // Time to live (duration)
    note: f32,    // Note/pitch
    gain: f32,    // Volume/gain
}

struct OscUniform {
    trucks: array<OscTruck, 16>,  // Corresponds to d1-d16
}
```

### 5. SpectrumUniform (When spectrum is configured)
- **Binding**: `@group(2) @binding(1)`
- **Content**: Audio spectrum analysis data

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
```

## Commonly Used Functions

### Color Conversion Functions

```wgsl
// Gamma correction
fn to_linear_rgb(col: vec3<f32>) -> vec3<f32> {
    let gamma = 2.2;
    let c = clamp(col, vec3(0.0), vec3(1.0));
    return pow(c, vec3(gamma));
}

// HSV to RGB conversion
fn hue_to_rgb(hue: f32) -> vec3<f32> {
    let kr = (5.0 + hue * 6.0) % 6.0;
    let kg = (3.0 + hue * 6.0) % 6.0;
    let kb = (1.0 + hue * 6.0) % 6.0;
    
    let r = 1.0 - max(min(min(kr, 4.0 - kr), 1.0), 0.0);
    let g = 1.0 - max(min(min(kg, 4.0 - kg), 1.0), 0.0);
    let b = 1.0 - max(min(min(kb, 4.0 - kb), 1.0), 0.0);
    
    return vec3(r, g, b);
}
```

### Shape Drawing Functions

```wgsl
// Draw light orb
fn orb(p: vec2<f32>, center: vec2<f32>, radius: f32, color: vec3<f32>) -> vec3<f32> {
    let t = clamp(1.0 + radius - length(p - center), 0.0, 1.0);
    return pow(t, 16.0) * color;
}

// Rectangle detection
fn rect(uv: vec2<f32>, pos: vec2<f32>, size: vec2<f32>) -> bool {
    let d = abs(uv - pos) - size;
    return max(d.x, d.y) < 0.0;
}

// Bar drawing (useful for spectrum visualization)
fn bar(uv: vec2<f32>, x: f32, width: f32, height: f32) -> bool {
    if (uv.x > x) && (uv.x < x + width) && (abs(uv.y) < height) {
        return true;
    }
    return false;
}
```

## Sample Projects

The included examples directory contains the following samples:

- `examples/mouse/`: Mouse-controlled shader art
- `examples/spectrum/`: Audio spectrum analysis visualizer
- `examples/osc/`: TidalCycles integration shader art

Use these as reference to create your own shader art projects.

## Uniform Binding Reference

### Group 0 (Always Available)
- `@binding(0)`: WindowUniform
- `@binding(1)`: TimeUniform

### Group 1 (Device Uniforms)
- `@binding(0)`: MouseUniform

### Group 2 (Sound Uniforms)
- `@binding(0)`: OscUniform (when OSC is configured)
- `@binding(1)`: SpectrumUniform (when spectrum is configured)