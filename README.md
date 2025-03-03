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
width = <Window width>
height = <Window height>

# pipeline is required
[[pipeline]]
shader_type = "fragment"
label = <Label of the shader>
entry_point = <Entry point of the shader>
file = <Path to the shader (wgsl) file>

# osc is required when handling messages from OSC (Open Sound Control)
# see examples/osc.toml for example
[osc]
port = <Port for OSC device>

# currently not supported. all OSC messages are handled
addr_pattern = <Address pattern to handle OSC messages>

[[osc.sound]]
name = <Sound name to handle>
id = <Number assigned for the sound in your shader>
```

## Fragment shaders

The following uniforms are available

```wgsl
struct WindowUniform {
    // window size in physical size
    resolution: vec2<f32>,
};

struct TimeUniform {
    // time elapsed since the program started
    duration: f32,
};

struct MouseUniform {
    // mouse position in physical size
    position: vec2<f32>,
};

// when osc is enabled
struct OscTruck {
    // OSC parameters for each OSC truck
    sound: i32,
    ttl: f32,
    note: f32,
    gain: f32,
}

// when osc is enabled
struct OscUniform {
    // OSC trucks (d1-d16), osc[0] for OSC d1
    trucks: array<OscTruck, 16>,
};

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;
@group(1) @binding(0) var<uniform> mouse: MouseUniform;

// when osc is enabled
@group(2) @binding(0) var<uniform> osc: OscUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Your shader codes here
}
```
