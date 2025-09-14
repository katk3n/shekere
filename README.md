# Shekere

<div align="center">
  <img src="shekere_logo.png" alt="Shekere Logo" width="480"/>
</div>

shekere is a real-time shader art framework that combines WGSL shaders with sound input. It supports mouse interaction, OSC control (TidalCycles, etc.), and audio spectrum analysis.

## Features

### 🎨 **Shader Support**
- **Fragment Shaders**: Real-time WGSL fragment shader execution
- **Multi-pass Rendering**: Complex effects with multiple rendering passes
  - **Sequential Passes**: Chain multiple shaders for layered effects
  - **Ping-pong Buffers**: Double-buffered textures for iterative algorithms
  - **Persistent Textures**: State preservation across frames
- **Hot Reload**: Live shader editing with automatic recompilation

### 🎵 **Audio Integration**
- **Spectrum Analysis**: Real-time FFT analysis of audio input
- **OSC Support**: Integration with TidalCycles and other OSC sources
- **MIDI Control**: Real-time MIDI input for interactive control
  - **Current Values**: Access current MIDI notes, controls, and note-on events
  - **Historical Data**: 512-frame history buffer for time-based effects
  - **Advanced Effects**: Fadeout, echo, trend analysis without multi-pass rendering

### 🛠️ **Development**
- **TOML Configuration**: Simple, human-readable project configuration
- **Built-in Uniforms**: Time, window, mouse, audio data automatically available
- **Helper Functions**: Color space conversion, coordinate helpers, audio accessors

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

### Example:

```bash
shekere examples/basic/basic.toml
```

## Getting Started

1. Create a TOML config file with window settings and shader path
2. Write a fragment shader using the built-in uniforms and helpers
3. Run with `shekere config.toml`

### MIDI History Features

Shekere v0.11.0 introduces powerful MIDI history functionality that enables sophisticated time-based effects without multi-pass rendering:

```wgsl
// Current MIDI values (compatible with previous versions)
let current_note = MidiNote(60u);        // Current Middle C velocity
let current_cc = MidiControl(7u);        // Current volume CC
let attack = MidiNoteOn(60u);            // Note attack detection

// New: Historical MIDI values
let past_note = MidiNoteHistory(60u, 10u);    // Middle C from 10 frames ago
let smooth_cc = MidiControlHistory(7u, 5u);   // Volume CC from 5 frames ago
let old_attack = MidiNoteOnHistory(60u, 20u); // Note attack from 20 frames ago

// Create fadeout effects with exponential decay
fn create_fadeout(note: u32) -> f32 {
    var intensity = 0.0;
    for (var h = 0u; h < 30u; h++) {
        let history_value = MidiNoteHistory(note, h);
        let decay = exp(-f32(h) * 0.1);
        intensity += history_value * decay;
    }
    return intensity / 30.0;
}
```

**Key Features:**
- **512-frame history buffer** with automatic bounds checking
- **Zero-cost current access** using `history = 0`
- **Backward compatible** with existing MIDI functions
- **Memory efficient** 768KB storage buffer design

## Documentation

For detailed documentation, see:

- **[Development Guide](docs/guide.md)** - Configuration, shader development, and examples
- **[API Reference](docs/api-reference.md)** - Complete reference for uniforms and helper functions

## Sample Projects

The included examples directory contains the following samples:

### Basic Examples
- `examples/basic/`: Basic time-based animation
- `examples/circular/`: Circular pattern with concentric rings
- `examples/mouse/`: Mouse-controlled shader art

### Audio Integration
- `examples/spectrum/`: Audio spectrum analysis visualizer
- `examples/osc/`: TidalCycles integration shader art
- `examples/midi/`: MIDI-controlled shader art
- `examples/midi_history/`: MIDI history effects (fadeout, echo, time series analysis)

### Advanced Multi-pass Shaders
- `examples/multi_pass/`: Multi-pass rendering with blur effects
- `examples/persistent/`: Persistent texture effects and trail rendering
- `examples/ping_pong/`: Ping-pong buffer simulation and kaleidoscope effects

Use these as reference to create your own shader art projects.