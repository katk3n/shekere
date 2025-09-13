# Code Style and Conventions

## Rust Conventions
- **Edition**: 2024
- **Formatting**: Use `cargo fmt` (MANDATORY before commits)
- **Linting**: Use `cargo clippy` for code quality
- **Error Handling**: Use `thiserror` for custom error types
- **Async**: Use `async-std` and `pollster::block_on` for async operations

## Naming Conventions
- **Structs**: PascalCase (e.g., `WindowUniform`, `SpectrumConfig`)
- **Functions**: snake_case (e.g., `fs_main`, `create_bind_group`)
- **Constants**: SCREAMING_SNAKE_CASE
- **Modules**: snake_case (e.g., `mouse_uniform`, `hot_reload`)

## Code Organization
- **Modular Uniforms**: Each uniform type in separate module under `src/uniforms/`
- **Config Structs**: Use serde derive for TOML parsing
- **Tests**: Integration tests in `tests/` directory, unit tests with `#[cfg(test)]`
- **Error Types**: Custom error enums with `thiserror::Error`

## WebGPU Patterns
- **Shader Loading**: Use `include_str!` for shader files
- **Bind Groups**: Dynamic creation based on enabled features
- **Buffer Alignment**: Use vec4 packing for WebGPU alignment requirements
- **Resource Management**: Proper cleanup and resource lifecycle management

## Documentation
- **Public APIs**: Use doc comments with examples
- **Modules**: Document module purpose and key types
- **Configuration**: Document all TOML options and their effects

## Memory Alignment
- **WebGPU Uniforms**: Pack data into vec4s for proper alignment
- **Buffer Usage**: Use `bytemuck` for safe byte casting
- **Padding**: Explicit padding fields where needed (e.g., `_padding: u32`)

## File Structure
- **Shaders**: Common definitions in `shaders/common.wgsl`
- **Examples**: Self-contained directories with TOML + shader files
- **Tests**: Comprehensive coverage with descriptive test names