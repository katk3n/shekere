# KaCHoFuGeTsu

kchfgt ("花鳥風月", which means beauties of nature) is a creative coding tool with shaders and sounds.

It's still under development.

## Install
```
cargo install kchfgt
```

## Usage

```
Creative coding tool with shaders and sounds

Usage: kchfgt [OPTIONS] <FILE>

Arguments:
  <FILE>  Input configuration file

Options:
  -h, --help             Print help
  -V, --version          Print version
```

## Config file format

```toml
# window is required
[window]
width = 800   # Window width
height = 800  # Window height

# pipeline is required
[[pipeline]]
shader_type = "fragment"   # Type of shader (currently only fragment is supported)
label = "Fragment Shader"  # Label of the shader
entry_point = "fs_main"    # Entry point of the shader
file = "fragment.wgsl"     # Path to the shader (wgsl) file

# See Uniforms section for other uniform settings

```

## Fragment shaders

The following uniforms are available

```wgsl
// Import Uniforms
// Other uniforms are also supported (see below)
struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
}

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
}

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Your shader codes here
}
```

## Uniforms

The following uniforms are supported.

### WindowUniform

#### Shader (wgsl)

```wgsl
@group(0) @binding(0) var<uniform> window: WindowUniform;

struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
};
```

### TimeUniform

#### Shader (wgsl)

```wgsl
@group(0) @binding(1) var<uniform> time: TimeUniform;

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
}
```

### MouseUniform

#### Shader (wgsl)

```wgsl
@group(1) @binding(0) var<uniform> mouse: MouseUniform;

struct MouseUniform {
    // mouse position in physical size
    position: vec2<f32>,
};
```

### OscUniform

OSC (Open Sound Control) which is used by Tidalcycles etc.
See [example](/examples/osc/).

#### Configuration

```toml
[osc]
port = 2020  # Port for OSC device

# currently not supported. all OSC messages are handled
addr_pattern = "/dirt/play"  # Address pattern to handle OSC messages

[[osc.sound]]
name = "bd"  # Sound name to handle
id = 1       # Number assigned for the sound in your shader

[[osc.sound]]
name = "sd"
id = 2
```

#### Shader (wgsl)

```wgsl
@group(2) @binding(0) var<uniform> osc: OscUniform;

struct OscTruck {
    // OSC parameters for each OSC truck
    sound: i32,
    ttl: f32,
    note: f32,
    gain: f32,
}

struct OscUniform {
    // OSC trucks (d1-d16), osc[0] for OSC d1
    trucks: array<OscTruck, 16>,
};
```

### SpectrumUniform

Audio spectrum analyzed by FFT.
See [example](/examples/spectrum/).

#### Configuration

```toml
[spectrum]
min_frequency = 27.0    # Min frequency to captrue
max_frequency = 2000.0  # Max frequency to capture
sampling_rate = 44100   # Sampling rate
```

#### Shader (wgsl)

```wgsl
@group(2) @binding(1) var<uniform> spectrum: SpectrumUniform;

struct SpectrumDataPoint {
    frequency: f32,
    amplitude: f32,
    // Not used but required to pass data to shader
    _padding: vec2<u32>,
}

struct SpectrumUniform {
    // spectrum data points of audio input
    data_points: array<SpectrumDataPoint, 2048>,
    // the number of data points
    num_points: u32,
    // frequency of the data point with the max amplitude
    max_frequency: f32,
    // max amplitude of audio input
    max_amplitude: f32,
}
```
