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

## Getting Started

1. Create a TOML config file with window settings and shader path
2. Write a fragment shader using the built-in uniforms and helpers
3. Run with `shekere config.toml`

## Documentation

For detailed documentation, see:

- **[Development Guide](docs/guide.md)** - Configuration, shader development, and examples
- **[API Reference](docs/api-reference.md)** - Complete reference for uniforms and helper functions

## Sample Projects

The included examples directory contains the following samples:

- `examples/mouse/`: Mouse-controlled shader art
- `examples/spectrum/`: Audio spectrum analysis visualizer
- `examples/osc/`: TidalCycles integration shader art
- `examples/midi/`: MIDI-controlled shader art

Use these as reference to create your own shader art projects.