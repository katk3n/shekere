# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

shekere is a Rust-based creative coding tool that combines WebGPU-based fragment shaders with audio integration (OSC and spectrum analysis). It creates real-time visual effects driven by sound and user interaction.

## Core Architecture

The application follows a modular architecture centered around the State pattern:

- **State (`src/state.rs`)**: Central state management handling WebGPU setup, uniforms, and render loop
- **Config (`src/config.rs`)**: TOML-based configuration system for window, shaders, OSC, and audio spectrum
- **Uniforms (`src/uniforms/`)**: Modular uniform system with separate modules for different data types:
  - `window_uniform.rs`: Window resolution data
  - `time_uniform.rs`: Time-based animation data
  - `mouse_uniform.rs`: Mouse position tracking
  - `osc_uniform.rs`: OSC (Open Sound Control) integration for Tidalcycles
  - `spectrum_uniform.rs`: Real-time audio spectrum analysis via FFT
- **Pipeline (`src/pipeline.rs`)**: WebGPU render pipeline creation and shader compilation
- **BindGroupFactory (`src/bind_group_factory.rs`)**: Dynamic bind group creation for different uniform combinations

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Run with a configuration file
cargo run -- examples/spectrum/spectrum.toml

# Build release version
cargo build --release
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Linting and Formatting
```bash
# Check code formatting
cargo fmt --check

# Format code
cargo fmt

# Run clippy lints
cargo clippy

# Run clippy with all targets
cargo clippy --all-targets
```

**MANDATORY BEFORE COMMITS**: Always run `cargo fmt` and ensure it completes without errors before creating any commits. Code formatting must be consistent across the entire codebase.

## Configuration System

The application uses TOML configuration files with the following structure:

- **Required**: `[window]` section with width/height
- **Required**: `[[pipeline]]` array with shader configuration
- **Optional**: `[osc]` for OSC integration with sound mapping
- **Optional**: `[spectrum]` for audio spectrum analysis

Example configuration files are in the `examples/` directory.

## Shader Development

Fragment shaders are written in WGSL (WebGPU Shading Language) and must:
- Use entry point `fs_main`
- Accept `VertexOutput` struct with `@builtin(position)`
- Return `@location(0) vec4<f32>` color output
- Access uniforms through predefined binding groups:
  - Group 0: Window and Time uniforms (always available)
  - Group 1: Device uniforms (mouse, etc.)
  - Group 2: Sound uniforms (OSC, spectrum, MIDI - when configured)
- Use helper functions for uniform data access (e.g., `SpectrumAmplitude(i)`, `OscGain(i)`)
- All sound uniforms use vec4 packing for WebGPU alignment optimization

## Audio Integration

The application supports two audio input methods:

1. **OSC Integration**: Receives OSC messages (typically from Tidalcycles) on configurable port
2. **Spectrum Analysis**: Real-time FFT analysis of system audio input with configurable frequency range

Both create GPU-accessible uniform data for shader consumption.

## Hot Reload System

The application includes a hot reload system for live coding:

- **File Watching**: Uses `notify` crate to monitor WGSL file changes
- **Error Safety**: WGSL compilation errors and pipeline creation errors are caught using `std::panic::catch_unwind()`
- **Graceful Degradation**: On error, the existing render pipeline is maintained and the application continues running
- **Auto Recovery**: After file modification, automatic reload is attempted
- **Configuration**: Enable with `[hot_reload] enabled = true`

## Key Implementation Details

- Uses `winit` for window management and input handling
- WebGPU backend selection via feature flags (PRIMARY backends on desktop, GL on WASM)
- Async initialization pattern for WebGPU setup
- Real-time uniform updates in the render loop
- Modular bind group system allows dynamic uniform combinations
- Configuration file path determines shader file resolution directory

## Testing Requirements

**MANDATORY**: When adding new features or modifying existing functionality:

1. **Write comprehensive unit tests**:
   - Test all public methods and functions
   - Cover both success and error cases
   - Include edge cases and boundary conditions
   - Use mock objects for external dependencies (file system, network, etc.)
   - Ensure tests are deterministic and not dependent on external state

2. **Add integration tests when appropriate**:
   - Test configuration parsing for new TOML sections
   - Test feature interactions with existing systems
   - Verify end-to-end workflows for complex features

3. **Follow testing patterns**:
   - Use `#[cfg(test)]` for test-only code and mocks
   - Place unit tests in the same file as the implementation
   - Place integration tests in `tests/` directory
   - Use descriptive test names that explain the scenario
   - Add helpful assertion messages for debugging

4. **Test coverage requirements**:
   - New features must have >90% test coverage
   - Critical paths (error handling, safety mechanisms) must have 100% coverage
   - Configuration parsing must be fully tested
   - Mock external dependencies for reliable testing

## Documentation Maintenance

**IMPORTANT**: When adding new features or modifying existing functionality:

1. **Always update README.md** to reflect changes in:
   - Configuration options and syntax
   - Available uniforms and their bindings
   - New shader development workflows
   - Example code and usage patterns

2. **Update this CLAUDE.md** with:
   - New architectural patterns or design decisions
   - Changes to core systems (uniforms, pipeline, etc.)
   - Important implementation details for future development

3. **Maintain consistency** between:
   - Configuration examples in README.md
   - Test cases in the codebase
   - Example projects in `examples/` directory