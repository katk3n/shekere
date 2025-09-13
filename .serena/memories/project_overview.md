# Shekere Project Overview

## Purpose
Shekere is a real-time shader art framework that combines WGSL (WebGPU Shading Language) shaders with sound input. It's a creative coding tool for live visual performances and installations.

## Key Features
- **Fragment Shaders**: Real-time WGSL fragment shader execution
- **Multi-pass Rendering**: Complex effects with sequential passes, ping-pong buffers, and persistent textures
- **Audio Integration**: Spectrum analysis, OSC control (TidalCycles), and MIDI input
- **Hot Reload**: Live shader editing with automatic recompilation
- **TOML Configuration**: Human-readable project configuration

## Tech Stack
- **Language**: Rust (2024 edition)
- **Graphics**: WebGPU (wgpu crate)
- **Window Management**: winit
- **Audio**: cpal (audio input), spectrum-analyzer (FFT)
- **OSC**: rosc (Open Sound Control)
- **MIDI**: midir
- **Configuration**: TOML/serde
- **File Watching**: notify (for hot reload)
- **CLI**: clap

## Architecture
- **State Pattern**: Centralized state management via `State` struct
- **Modular Uniforms**: Separate modules for different uniform types (window, time, mouse, OSC, spectrum, MIDI)
- **Configuration-Driven**: TOML files determine application structure
- **WebGPU Backend**: Cross-platform graphics with automatic backend selection

## Project Structure
- `src/` - Main source code
  - `uniforms/` - Modular uniform system
  - `main.rs` - Entry point
  - `state.rs` - Central state management
  - `config.rs` - TOML configuration parsing
  - `pipeline.rs` - WebGPU render pipeline
- `shaders/` - Common WGSL shader definitions
- `examples/` - Sample projects and configurations
- `tests/` - Integration and unit tests
- `docs/` - Documentation and design specs