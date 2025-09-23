# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

shekere is a Rust-based creative coding tool that combines WebGPU-based fragment shaders with audio integration (OSC and spectrum analysis). It creates real-time visual effects driven by sound and user interaction.

## Workspace Architecture

shekere is organized as a Rust workspace with three main packages:

- **shekere-core**: Core library containing WebGPU rendering engine, audio integration, shader management, and uniform systems
- **shekere-cli**: Command-line application that provides the traditional shekere experience
- **shekere-gui**: Desktop GUI application built with Tauri and Svelte for visual shader development

### Package Dependencies
- Both CLI and GUI depend on shekere-core
- All packages share workspace-level dependencies for consistency
- shekere-core is designed to be embeddable and window-system independent

For detailed architecture specifications, see [docs/design/architecture.md](docs/design/architecture.md).

## Development Commands

### Build and Run
```bash
# Build the entire workspace
cargo build

# Build specific packages
cargo build --package shekere-core
cargo build --package shekere-cli
cargo build --package shekere-gui

# Run CLI with a configuration file
cargo run --bin shekere-cli -- examples/spectrum/spectrum.toml

# Run GUI application
cargo run --bin shekere-gui

# Build release versions
cargo build --release
cargo build --release --bin shekere-cli
cargo build --release --bin shekere-gui
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

For detailed configuration specifications, see [docs/design/configuration.md](docs/design/configuration.md).

Example configuration files are in the `examples/` directory.

## Shader Development

**MANDATORY**: When creating or modifying shaders,
 see [docs/guide.md](docs/guide.md) and [docs/api-reference.md](docs/api-reference.md).

**Always check `shaders/common.wgsl` first** and
DO NOT redefine structures, uniforms, or functions that already exist in `common.wgsl`

## Audio Integration

For detailed audio integration specifications, see:
- [OSC Integration](docs/design/osc.md)
- [MIDI Integration](docs/design/midi.md)
- [Spectrum Analysis](docs/design/spectrum.md)

## Hot Reload System

For detailed hot reload system specifications, see [docs/design/hot-reload.md](docs/design/hot-reload.md).


## Development Methodology

**MANDATORY**: All development must follow Test-Driven Development (TDD) methodology as advocated by t-wada:

1. **Red-Green-Refactor Cycle**:
   - Write a failing test first (Red)
   - Write the minimal code to make the test pass (Green)
   - Refactor the code while keeping tests passing (Refactor)

2. **Test-First Approach**:
   - Never write production code without a failing test
   - Write tests that describe the behavior you want to implement
   - Use tests as design documentation and specifications

3. **Small, Incremental Steps**:
   - Make small, focused changes in each cycle
   - Commit after each successful Red-Green-Refactor cycle
   - Build functionality incrementally through TDD cycles

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
