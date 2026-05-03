# Architecture Decision Record (ADR): TSL and WebGPU Support

Comprehensive record of the successful migration of Shekere to Three.js Shading Language (TSL) and WebGPURenderer.

## 1. Status

Implemented (May 2026)

## 2. Context & Goals

Legacy GLSL-based `ShaderMaterial` and `EffectComposer` create high friction with the modern Three.js ecosystem and lack compatibility with modern WebGPU pipelines. The goal was to pivot the entire visualizer core to a unified, JavaScript-first shader authoring experience using TSL and native WebGPU rendering, ensuring long-term performance and maintainability.

## 3. Implemented Architecture

### 3.1 Rendering Engine
- **WebGPURenderer**: Replaces `WebGLRenderer`. Automatically falls back to WebGL 2 if WebGPU is not supported by the environment, but maintains the modern node-based logic.
- **THREE.Timer**: Replaces the deprecated `THREE.Clock` for precise render loop timing in modern Three.js.

### 3.2 TSL-Native Post-Processing
- **RenderPipeline**: Replaces legacy `EffectComposer` for post-processing execution.
- **Node-Based FX Chain**: All core visual effects (Bloom, RGB Shift, Film Grain, Vignette) are implemented entirely using TSL node functions (`bloom()`, `rgbShift()`, `film()`). 
- **Direct Node Parameter Mutation**: For dynamic audio-reactivity and UI control, the architecture explicitly avoids passing custom `UniformNode` wrappers into Three.js utility functions (which silently strip/clone them). Instead, it injects primitive values during initialization and dynamically mutates the generated node properties (e.g., `(bloomNode as any).strength.value = v`) or explicitly provides mapped node variables where supported (e.g., `FilmNode` intensity).

### 3.3 Geometry and Primitives
- **InstancedMesh Adoption**: `THREE.Points` is heavily restricted in WebGPU (point sizes are clamped to 1 pixel). Point cloud visuals (e.g., `shader_stars.js`) have been migrated to `THREE.InstancedMesh` using `PlaneGeometry` combined with custom TSL vertex transformations (`vertexNode` billboard math) to support scalable, GPU-driven particles.

## 4. Challenges Overcome

- **API Rapid Flux**: Addressed deprecation warnings and structural changes in Three.js r183 (e.g., `PostProcessing` -> `RenderPipeline`, `renderAsync()` -> `await renderer.init()`).
- **Post-Processing Graph Topology**: Solved white-out/clipping errors by properly adding the isolated bloom output back into the scene color pass (`scenePass.getTextureNode('output').add(bloomPass)`), ensuring accurate HDR blending.
- **Tone Mapping Conflicts**: Maintained `NoToneMapping` as the framework default to preserve the high-intensity neon glow of the post-processing pipeline. Tone mapping severely compressed `>1.0` emissive intensity bursts, which was contrary to the intended artistic style.
- **Vignette Logic**: Recreated the standard "Eskil's Vignette" formula purely in TSL to guarantee consistent UI mapping (Darkness/Offset) identical to the legacy `VignetteShader`.

## 5. Consequences

- **Performance**: Access to modern WebGPU compute workflows and optimized shader node compilation.
- **Exclusivity**: Support for legacy GLSL injection in `ShaderMaterial` is officially sunset in the core engine. All custom sketches must utilize TSL for procedural materials and effects.
- **Maintainability**: Unified rendering pipeline with cleaner, chainable node syntax instead of complex pass management.
