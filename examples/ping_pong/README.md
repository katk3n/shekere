# Ping-Pong Buffer Examples

This directory contains examples demonstrating ping-pong buffer functionality in shekere. Ping-pong buffers are perfect for stateful shaders that need to read from the previous frame's output, enabling complex simulations and feedback effects.

## What are Ping-Pong Buffers?

Ping-pong buffers use two textures that alternate roles between frames:
- **Frame N**: Render to Buffer A, read from Buffer B  
- **Frame N+1**: Render to Buffer B, read from Buffer A
- **Frame N+2**: Render to Buffer A, read from Buffer B
- And so on...

This technique allows shaders to maintain state between frames, enabling simulations like cellular automata, reaction-diffusion systems, and feedback effects.

## Configuration

To use ping-pong buffers, add `ping_pong = true` to your shader configuration:

```toml
[[pipeline]]
shader_type = "fragment"
label = "My Stateful Shader"
entry_point = "fs_main"
file = "my_shader.wgsl"
ping_pong = true
```

## Available Functions

In your shaders, use these functions to read from the previous frame:

```wgsl
// Sample from the previous frame at the given UV coordinates
let previous_color = SamplePreviousPass(uv);

// Sample with an offset (useful for neighbor sampling)
let neighbor = SamplePreviousPassOffset(uv, vec2<f32>(1.0/width, 0.0));
```

## Examples

### 1. Game of Life (`game_of_life.toml`)

Conway's Game of Life implemented as a ping-pong shader.

**Features:**
- Classic cellular automaton rules
- Mouse interaction to add new cells
- Toroidal topology (wrapping edges)
- Initial patterns including gliders and oscillators
- Visual enhancements with neighbor count coloring

**Controls:**
- Click and drag to add living cells
- Watch patterns evolve over time

**Run with:**
```bash
cargo run -- examples/ping_pong/game_of_life.toml
```

### 2. Reaction-Diffusion (`reaction_diffusion.toml`)

Gray-Scott reaction-diffusion system creating organic, coral-like patterns.

**Features:**
- Two-chemical system (A and B)
- Configurable diffusion and reaction rates
- Mouse interaction to seed new reactions
- Beautiful color mapping based on chemical concentrations
- Multiple seed points for interesting initial conditions

**Controls:**
- Click and drag to add chemical B (creates new reaction sites)
- Watch complex patterns emerge and evolve

**Run with:**
```bash
cargo run -- examples/ping_pong/reaction_diffusion.toml
```

## Technical Details

### Memory Usage
Each ping-pong shader uses two full-screen textures, doubling the memory footprint compared to regular shaders. For a 1920x1080 display with RGBA8 format, this means ~16MB per ping-pong shader.

### Performance Considerations
- Ping-pong shaders render to intermediate textures, then copy to screen
- Each frame processes the entire texture, regardless of how much has changed
- GPU memory bandwidth is the primary bottleneck
- Consider reducing resolution for complex simulations

### Shader Requirements
Your ping-pong shaders should:
1. Always sample from `SamplePreviousPass(uv)` to get the previous frame
2. Handle edge cases gracefully (UV coordinates outside [0,1])
3. Initialize properly on the first few frames
4. Avoid reading and writing to the same location simultaneously

### Debugging Tips
1. **Black screen**: Check if you're properly reading from `SamplePreviousPass()`
2. **No evolution**: Ensure you're not just copying the previous frame
3. **Noise/artifacts**: Check UV coordinate calculations and sampling
4. **Performance issues**: Monitor GPU memory usage and consider resolution

## Advanced Patterns

### Neighbor Sampling
```wgsl
// Count living neighbors in Game of Life
var neighbors = 0u;
for (var dx = -1; dx <= 1; dx++) {
    for (var dy = -1; dy <= 1; dy++) {
        if (dx == 0 && dy == 0) { continue; }
        let neighbor_uv = uv + vec2<f32>(f32(dx), f32(dy)) * cell_size;
        let neighbor_state = SamplePreviousPass(neighbor_uv);
        if (neighbor_state.r > 0.5) {
            neighbors += 1u;
        }
    }
}
```

### Convolution Kernels
```wgsl
// Laplacian operator for diffusion
var laplacian = 0.0;
let kernel = array<f32, 9>(
    0.05, 0.2, 0.05,
    0.2,  -1.0, 0.2,
    0.05, 0.2, 0.05
);

for (var i = 0; i < 3; i++) {
    for (var j = 0; j < 3; j++) {
        let offset = vec2<f32>(f32(i - 1), f32(j - 1)) * texel_size;
        let sample_uv = uv + offset;
        let neighbor = SamplePreviousPass(sample_uv);
        laplacian += neighbor.r * kernel[i * 3 + j];
    }
}
```

### Frame-Based Initialization
```wgsl
// Initialize on early frames only
if (Time.frame < 10) {
    // Set up initial conditions
    return initial_pattern(uv);
} else {
    // Normal simulation
    let previous = SamplePreviousPass(uv);
    return simulate_step(previous, uv);
}
```

## Creating Your Own Ping-Pong Shaders

1. **Start Simple**: Begin with a shader that just copies the previous frame
2. **Add State**: Introduce small changes based on the previous frame
3. **Test Boundaries**: Ensure your shader handles edge cases properly
4. **Optimize**: Profile and optimize for your target resolution and framerate

Example minimal ping-pong shader:
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.tex_coords;
    let previous = SamplePreviousPass(uv);
    
    // Simple feedback effect: fade over time
    return previous * 0.99;
}
```

## Further Reading

- [Ping-Pong Technique on GPU Gems](https://developer.nvidia.com/gpugems/gpugems2/part-v-image-oriented-computing/chapter-37-octree-textures-gpu)
- [Reaction-Diffusion Systems](https://en.wikipedia.org/wiki/Reaction%E2%80%93diffusion_system)
- [Conway's Game of Life](https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life)
- [GPU-based Cellular Automata](https://www.karlsims.com/rd.html)