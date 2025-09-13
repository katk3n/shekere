# Development Patterns and Guidelines

## Key Design Patterns

### State Pattern
- Central `State` struct manages all application state
- WebGPU resources, uniforms, and render loop coordination
- Async initialization pattern for WebGPU setup

### Modular Uniforms System
- Separate modules for each uniform type in `src/uniforms/`
- Dynamic bind group creation based on enabled features
- vec4 packing for WebGPU alignment optimization
- Configuration-driven uniform combinations

### Configuration-Driven Architecture
- TOML files determine application structure
- No code changes needed for different deployment scenarios
- Flexible pipeline definitions and feature toggles

## WebGPU Best Practices
- **Backend Selection**: Automatic selection based on platform
- **Resource Lifecycle**: Proper cleanup and management
- **Buffer Alignment**: Use vec4 packing for uniforms
- **Bind Group Factory**: Dynamic creation for different uniform combinations

## Audio Integration Patterns
- **Non-blocking Audio**: Separate thread for audio processing
- **Ring Buffers**: Use `ringbuf` for thread-safe audio data sharing
- **FFT Processing**: Real-time spectrum analysis with `spectrum-analyzer`
- **OSC Integration**: Async OSC server with `rosc` and `async-std`

## Hot Reload System
- **File Watching**: Use `notify` crate for filesystem monitoring
- **Error Recovery**: Graceful handling of shader compilation errors
- **State Preservation**: Maintain application state during reloads
- **Multi-pass Support**: Hot reload for complex pipeline configurations

## Testing Strategies
- **Integration Tests**: Full pipeline testing with mock WebGPU
- **Configuration Testing**: TOML parsing and validation
- **Hot Reload Testing**: File change simulation and error handling
- **Render Testing**: Baseline comparison for visual regression

## Error Handling
- **Custom Error Types**: Use `thiserror` for domain-specific errors
- **Graceful Degradation**: Continue operation when non-critical features fail
- **User-Friendly Messages**: Clear error reporting for configuration issues
- **Recovery Mechanisms**: Attempt to recover from shader compilation errors

## Memory Management
- **Resource Pooling**: Reuse WebGPU resources where possible
- **Texture Management**: Efficient handling of multi-pass textures
- **Buffer Reuse**: Minimize allocation/deallocation overhead
- **Cleanup Patterns**: Proper Drop implementations for resources