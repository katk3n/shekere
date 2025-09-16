# Architecture Specification

## Overview

shekere is a Rust-based creative coding tool that combines WebGPU-based fragment shaders with audio integration (OSC and spectrum analysis). It creates real-time visual effects driven by sound and user interaction.

## Core Architecture

The application follows a modular architecture centered around the State pattern:

### Core Components

- **State (`src/state.rs`)**: Central state management handling WebGPU setup, uniforms, and render loop
- **Config (`src/config.rs`)**: TOML-based configuration system for window, shaders, OSC, and audio spectrum
- **Uniforms (`src/uniforms/`)**: Basic uniform data types:
  - `window_uniform.rs`: Window resolution data
  - `time_uniform.rs`: Time-based animation data
- **Inputs (`src/inputs/`)**: Complex input processing with history support:
  - `mouse.rs`: Mouse input with 512-frame history
  - `osc.rs`: OSC (Open Sound Control) integration with history
  - `spectrum.rs`: Real-time audio spectrum analysis via FFT with history
  - `midi.rs`: MIDI input processing with history
- **Pipeline (`src/pipeline.rs`)**: WebGPU render pipeline creation and shader compilation
- **BindGroupFactory (`src/bind_group_factory.rs`)**: Dynamic bind group creation for different uniform combinations

## Key Implementation Details

- Uses `winit` for window management and input handling
- WebGPU backend selection via feature flags (PRIMARY backends on desktop, GL on WASM)
- Async initialization pattern for WebGPU setup
- Real-time uniform updates in the render loop
- Modular bind group system allows dynamic uniform combinations
- Configuration file path determines shader file resolution directory

## Design Patterns

### State Pattern
The application uses a centralized state management pattern where the `State` struct manages all application state including WebGPU resources, uniforms, and render loop coordination.

### Modular Uniforms
The uniform system is designed to be modular, allowing different combinations of uniforms to be used based on configuration. This is achieved through:
- Separate modules for each uniform type
- Dynamic bind group creation based on enabled features
- vec4 packing for WebGPU alignment optimization

### Configuration-Driven Architecture
The application structure is determined by TOML configuration files, allowing for flexible deployment scenarios without code changes.