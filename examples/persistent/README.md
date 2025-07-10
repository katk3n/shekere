# Persistent Texture Trail Effect

This example demonstrates the `persistent = true` functionality in shekere's multi-pass shader system. Two versions are provided:

- **`persistent.toml` + `trail.wgsl`**: Improved trail effect with longer, more visible trails
- **`persistent_enhanced.toml` + `trail_enhanced.wgsl`**: Dramatic trail effect with 3 objects and vivid colors

## What it does

The shader creates a beautiful trail effect by:

1. **Persistent State**: Using `persistent = true` to preserve texture content between frames
2. **Fade Effect**: Each frame, the previous content is multiplied by 0.95 to create a gradual fade
3. **New Content**: Adding new bright spots that move in circular patterns
4. **Color Variation**: The spots change color over time using sine/cosine functions
5. **Background Patterns**: Subtle animated background patterns for visual interest

## Key Features

- **Frame Persistence**: Previous frame content is automatically available as input
- **Accumulation**: New content is added to faded previous content each frame
- **Smooth Trails**: Moving objects leave smooth, fading trails behind them
- **Time-based Animation**: Uses the `time.duration` uniform for smooth animation

## Running the Examples

**Basic improved trail effect:**
```bash
cargo run -- examples/persistent/persistent.toml
```

**Enhanced dramatic trail effect:**
```bash
cargo run -- examples/persistent/persistent_enhanced.toml
```

## Technical Details

### Configuration
- Uses `persistent = true` flag in the shader configuration
- Single-pass shader that reads from its own previous frame output
- Automatic texture management handles the persistent state

### Shader API
- `SamplePreviousPass(uv)` - Samples the texture from the previous frame
- Built-in `time.duration` uniform for animation timing
- Standard `VertexOutput` with `tex_coords` for UV mapping

### Performance
- Efficient single-pass rendering with automatic texture switching
- No manual state management required
- Minimal performance overhead compared to basic shaders

## Visual Effect

The result is a mesmerizing trail effect where bright colored spots move around the screen, leaving beautiful fading trails behind them. The background also features subtle animated patterns that add depth to the visual.

This demonstrates how persistent textures enable stateful shader art that builds up complex visuals over time.