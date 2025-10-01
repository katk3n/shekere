# Hot Reload System Specification

## Overview

The hot reload system enables live coding and development workflows by automatically recompiling and reloading shaders when source files are modified. This system is designed for safety and reliability, ensuring the application continues running even when shader compilation fails.

## Core Features

### File Watching
- Uses the `notify` crate to monitor WGSL file changes
- Automatically detects file modifications in real-time
- Supports multiple shader files simultaneously

### Error Safety
- **Shader validation** performed before compilation (syntax checks)
- **Existing shader protection**: Failed reloads keep the current working shader
- **Pipeline creation errors** are handled gracefully
- Error handling ensures the application **never crashes** and **never shows a black screen** due to shader issues

### Graceful Degradation
- On compilation error, the existing render pipeline is maintained
- Application continues running with the last successful shader
- Visual feedback or logging indicates compilation failures

### Auto Recovery
- After file modification, automatic reload is attempted
- No manual intervention required for successful recompilation
- Seamless transition to updated shaders when compilation succeeds

## Configuration

### Enable Hot Reload
```toml
[hot_reload]
enabled = true
```

### Disable Hot Reload
```toml
[hot_reload]
enabled = false
```

Or simply omit the `[hot_reload]` section entirely (defaults to disabled).

## Development Workflow

### Live Coding Support
- Modify shader files in real-time
- See changes reflected immediately in the application
- Iterate quickly on visual effects without restarting

### Error Handling Workflow
1. Developer modifies shader file
2. Hot reload system detects file change
3. Shader compilation is attempted
4. If successful: New pipeline is created and activated
5. If failed: Error is logged, existing pipeline continues

### Performance Considerations
- File watching has minimal performance impact
- Shader compilation only occurs on file changes
- No impact on render loop performance

## Implementation Details

### Bevy ECS Architecture (v0.13.0+)

The hot reload system is fully integrated with Bevy's ECS architecture:

#### Resource-Based Management
- **`HotReloaderResource`**: Single-pass hot reload state
- **`MultiPassState`**: Multi-pass hot reload with per-pass shader tracking
- **`PersistentPassState`**: Persistent/ping-pong rendering hot reload

#### System Integration
Three dedicated systems run in Bevy's Update schedule:
1. **`check_shader_reload()`**: Single-pass shader hot reload
2. **`check_multipass_shader_reload()`**: Multi-pass shader hot reload (runs conditionally)
3. **`check_persistent_shader_reload()`**: Persistent/ping-pong shader hot reload (runs conditionally)

#### File Watching
- Uses `notify` crate with `HotReloader` struct
- Separate `HotReloader` instance for each rendering mode
- File changes detected via `check_for_changes()` method

#### Shader Regeneration with Validation
- **Single-pass**:
  - Regenerates via `generate_clean_shader_source()`
  - Validates before applying
  - Checks existing shader exists before overwriting
- **Multi-pass**:
  - Regenerates all passes via `generate_shader_for_pass()`
  - Validates ALL passes before updating ANY (atomic operation)
  - Maintains pipeline consistency
- **Persistent**:
  - Regenerates via `generate_clean_shader_source()`
  - Validates before applying
  - Preserves texture buffer state on errors

#### Validation Checks
The `validate_shader_source()` function performs:
1. **Empty source check**: Ensures shader has content
2. **Fragment function check**: Verifies `fn fragment()` or `fn fs_main()` exists
3. **Brace balance check**: Validates matching `{` and `}` counts
4. **Required imports check**: Ensures `VertexOutput` is present

### Thread Safety
- File watching runs on separate threads (via `notify` crate)
- Shader compilation is isolated from render thread
- Thread-safe communication between file watcher and Bevy systems

### Memory Management
- Old shader assets replaced via `shaders.insert(&handle, new_shader)`
- Bevy's asset system handles cleanup automatically
- Resource management prevents memory leaks during development cycles

### Integration with State Management
- Hot reload integrates with Bevy's resource system
- Pipeline updates coordinated with uniform updates via Update schedule
- Maintains consistency between shader and uniform data

## Multi-Pass Hot Reload Support

### Overview
The hot reload system fully supports multi-pass shader configurations, including:
- Traditional multi-pass rendering (multiple shader files)
- Ping-pong buffer configurations 
- Persistent texture configurations

### Multiple File Watching
- All shader files in the pipeline are monitored simultaneously
- Changes to any shader file trigger a complete pipeline reconstruction
- Maintains consistency across all pipeline stages

### Example Configurations

#### Multi-Pass Configuration
```toml
[hot_reload]
enabled = true

[[pipeline]]
label = "Scene"
file = "scene.wgsl"
shader_type = "fragment"
entry_point = "fs_main"

[[pipeline]]
label = "Blur"
file = "blur.wgsl"
shader_type = "fragment"
entry_point = "fs_main"
```

#### Ping-Pong Configuration
```toml
[hot_reload]
enabled = true

[[pipeline]]
label = "Game of Life"
file = "life.wgsl"
shader_type = "fragment"
entry_point = "fs_main"
ping_pong = true
```

#### Persistent Texture Configuration
```toml
[hot_reload]
enabled = true

[[pipeline]]
label = "Trail Effect"
file = "trail.wgsl"
shader_type = "fragment"
entry_point = "fs_main"
persistent = true
```

### Technical Implementation

#### Pipeline Recreation
- Entire `MultiPassPipeline` is recreated on shader changes
- Bind group layouts are reconstructed to ensure compatibility
- Texture manager state is cleared to avoid conflicts

#### State Preservation
- Texture contents are cleared on reload to ensure clean state
- Uniform data is preserved across pipeline changes
- Audio and interaction state continues uninterrupted

#### Error Handling
- Compilation errors in any shader file prevent pipeline update
- Application continues with existing functional pipeline
- Detailed error logging helps identify specific issues

### Development Benefits

#### Rapid Iteration
- Modify any shader in a multi-pass pipeline
- See changes immediately without restart
- Test complex multi-pass effects in real-time

#### Complex Effect Development
- Develop ping-pong simulations interactively
- Fine-tune persistent texture effects live
- Experiment with multi-stage rendering pipelines

#### Debugging Support
- Isolate issues to specific pipeline stages
- Test individual shader modifications
- Maintain application state during development