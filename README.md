# KaCHoFuGeTsu

kchfgt ("花鳥風月", which means beauties of nature) is a creative coding tool with shaders and sounds.

It's still under development.

## Usage

```
Creative coding tool with shaders and sounds

Usage: kchfgt [OPTIONS] <FILE>

Arguments:
  <FILE>  Input fragment shader file. Only wgsl is supported

Options:
      --width <WIDTH>    Window width [default: 1280]
      --height <HEIGHT>  Window height [default: 720]
  -h, --help             Print help
  -V, --version          Print version
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

@group(0) @binding(0) var<uniform> window: WindowUniform;
@group(0) @binding(1) var<uniform> time: TimeUniform;
@group(1) @binding(0) var<uniform> mouse: MouseUniform;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Your shader codes here
}
```
