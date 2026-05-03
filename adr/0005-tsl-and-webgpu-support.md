# Architecture Decision Record (ADR): TSL and WebGPU Support (Attempted & Postponed)

Comprehensive record of the attempt to migrate Shekere to Three.js Shading Language (TSL) and WebGPURenderer.

## 1. Status

Postponed (Implementation attempted in May 2026, reverted to maintain stability)

## 2. Context & Goals

Legacy GLSL-based `ShaderMaterial` and `EffectComposer` create high friction with the modern Three.js ecosystem. The goal was to pivot to a unified, JavaScript-first shader authoring experience using TSL.

## 3. Proposed Architecture (The "TSL Edition" Design)

The following design was implemented during the investigation:

### 3.1 Rendering Engine
- **WebGPURenderer**: Replaces `WebGLRenderer` to enable native Node support.
- **PointsNodeMaterial / MeshStandardNodeMaterial**: Replaces standard materials to allow direct TSL node injection.

### 3.2 TSL-Native Post-Processing
- **RenderPipeline**: Replaces legacy `EffectComposer`.
- **Node-Based Effects**: Implementing visual effects (Bloom, RGB Shift, Film Grain) as TSL nodes instead of standard ShaderPasses.
- **Sampling Logic**: Using `pass.textureNode.sample(uv)` or similar TSL constructs for sampling previous render passes.

### 3.3 Global API Contract
- `window.TSL`: Global exposure of all TSL functions.
- `window.MeshStandardNodeMaterial`, etc.: Direct exposure of node-ready material classes.
- Unified Audio/MIDI uniforms integrated directly into TSL graphs.

## 4. Challenges & Technical Hurdles

Despite the design's benefits, the implementation revealed critical challenges with the current state of Three.js (r183):

- **API Rapid Flux**: Core classes and methods were renamed mid-development (e.g., `PostProcessing` -> `RenderPipeline`, `timerLocal()` -> `timer`).
- **Import Fragility**: Static imports for post-processing nodes like `bloom` caused fatal `SyntaxError` in Vite due to package export issues. Dynamic detection was required as a workaround.
- **Fragment Module Invalidity**: Using TSL with `THREE.Points` requires the specific `PointsNodeMaterial`, otherwise the fragment shader generation fails silently.
- **GLSL Compatibility Gap**: `WebGPURenderer` (even in WebGL 2 fallback mode) does not natively support legacy `ShaderMaterial` without explicit conversion nodes, threatening existing user sketches.

## 5. Decision & Current Action

1. **Revert to WebGL 2**: Source code has been reverted to the stable `WebGLRenderer` and `EffectComposer` pipeline to ensure all existing sketches (GLSL) function correctly.
2. **Preserve Design**: This ADR serves as the blueprint for future TSL support.
3. **Deferment Criteria**: Re-evaluation will occur once Three.js stabilizes the `RenderPipeline` API and provides a more robust GLSL-to-Node compatibility layer.

## 6. Consequences

- **Stability**: Shekere remains a reliable tool for current live coding workflows.
- **Debt**: The project continues to rely on legacy rendering techniques (WebGL 1/2), missing out on WebGPU performance gains.
- **Future Readiness**: The "TSL Edition" logic (Vignette/RGB Shift nodes) and the specialized material findings are now documented and can be reactivated quickly.
