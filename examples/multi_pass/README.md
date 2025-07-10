# Multi-Pass Rendering Example

This example demonstrates the basic multi-pass rendering functionality in shekere.

## What it does

1. **Scene Pass**: Creates a colorful animated spiral pattern with rainbow colors
2. **Blur Pass**: Applies a 5x5 box blur effect to the scene from the first pass

## Configuration

The `multi_pass.toml` file defines two pipeline stages:
- `scene.wgsl`: Generates the initial colorful pattern
- `blur.wgsl`: Applies blur using the `SamplePreviousPass()` function

## Running

```bash
cargo run -- examples/multi_pass/multi_pass.toml
```

## How it works

- The first pass renders to an intermediate texture (`temp_0`)
- The second pass samples from that intermediate texture and applies blur
- The final result is rendered to the screen

This demonstrates the automatic texture management and Group 3 binding system for multi-pass shaders.