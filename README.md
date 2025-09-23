# Shekere

<div align="center">
  <img src="shekere_logo.png" alt="Shekere Logo" width="480"/>
</div>

shekere is a real-time shader art framework that combines WGSL shaders with sound input. It supports mouse interaction, OSC control (TidalCycles, etc.), and audio spectrum analysis.

## Architecture

shekere is built as a Rust workspace with three main components:

- **shekere-core**: Core library providing WebGPU rendering, audio integration, and shader management
- **shekere-cli**: Command-line application for running shader projects
- **shekere-gui**: Desktop GUI application built with Tauri for visual shader development

This modular architecture allows shekere to be embedded in other applications or used as a standalone tool.

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

## Installation & Usage

shekere is available as both a command-line tool and a GUI application.

### Command Line Interface (CLI)

Build and run from source:

```bash
git clone https://github.com/katk3n/shekere.git
cd shekere
cargo run --bin shekere-cli -- examples/basic/basic.toml
```

Or build a release binary:

```bash
cargo build --release --bin shekere-cli
./target/release/shekere-cli examples/basic/basic.toml
```

### Graphical User Interface (GUI)

Build and run the GUI application:

```bash
cargo run --bin shekere-gui
```

The GUI provides:
- Visual project browser and file management
- Live shader editing with preview
- Real-time configuration editing
- Built-in examples and templates

## Getting Started

1. Create a TOML config file with window settings and shader path
2. Write a fragment shader using the built-in uniforms and helpers
3. Run with `cargo run --bin shekere-cli config.toml` or open in the GUI

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