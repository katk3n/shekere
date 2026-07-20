# 0009: Host-Managed GPU Feedback Passes

## Status

Implemented

## Date

2026-07-19

## Context

Shekere sketches can create Three.js and TSL materials and can react directly
to audio, MIDI, OSC, time, and the camera texture. ADR 0008 additionally
provides motion-mask and motion-trail textures. These inputs are sufficient
for effects whose output depends only on the current inputs or on the trail
already maintained by the camera motion analyzer.

Some effects require their own evolving GPU state. Examples include ripples
that continue expanding after motion stops, smoke or fluid-like feedback,
patterns that accumulate over time, and particles whose position, velocity,
and lifetime persist independently of the source motion. These effects require
reading a previous output while writing a new output, normally with ping-pong
render targets.

Exposing `WebGPURenderer` or raw render targets directly to sketches would let
sketches corrupt host render state, sample and render to the same texture,
interfere with post-processing, leak GPU resources across hot reloads, or
dispose resources owned by another subsystem. Adding an orb-, particle-, or
ripple-specific host API would be safer but too narrow for a live-coding
environment.

Shekere therefore needs a generic, host-scheduled GPU feedback capability. It
must let sketches define TSL state transitions while the host retains renderer
access, execution ordering, validation, and resource ownership.

## Decision

### 1. Add a generic `Shekere.gpu` feedback API

The Visualizer will add a host-owned GPU service to the existing global
`Shekere` namespace:

```typescript
interface FeedbackPassOptions {
  name?: string;
  width: number;
  height: number;
  format?: "rgba8" | "rgba16f";
  clearValue?: [number, number, number, number];
  textures?: string[];
  uniforms?: Record<string, FeedbackUniformValue>;
  build: (context: FeedbackBuildContext) => TSL.Node;
}

interface FeedbackBuildContext {
  previous: TSL.TextureNode;
  textures: Record<string, TSL.TextureNode>;
  uniforms: Record<string, TSL.UniformNode>;
  uv: TSL.Node;
  deltaTime: TSL.UniformNode;
  time: TSL.UniformNode;
}

type FeedbackUniformValue =
  | number
  | [number, number]
  | [number, number, number]
  | [number, number, number, number];

type FeedbackTextureInput =
  | THREE.Texture
  | TSL.TextureNode
  | FeedbackPass
  | null;

interface FeedbackPassUpdate {
  textures?: Record<string, FeedbackTextureInput>;
  uniforms?: Record<string, FeedbackUniformValue>;
}

interface FeedbackPass {
  readonly node: TSL.TextureNode;
  readonly texture: THREE.Texture | null;
  readonly width: number;
  readonly height: number;
  update(values?: FeedbackPassUpdate): void;
  clear(): void;
  dispose(): void;
}

interface ShekereGpuApi {
  createFeedbackPass(options: FeedbackPassOptions): FeedbackPass;
}
```

Example using ADR 0008 motion as the seed for state that keeps spreading after
the source motion stops:

```javascript
export function setup(scene) {
  this.scene = scene;
  this.previousBackgroundNode = scene.backgroundNode;
  this.ripple = null;
  this.rippleWidth = 0;
  this.rippleHeight = 0;

  return {
    camera: { motion: { enabled: true } }
  };
}

export function update({ camera, audio }) {
  if (!camera.motion.active) return;

  const width = camera.motion.width;
  const height = camera.motion.height;
  if (!this.ripple || width !== this.rippleWidth || height !== this.rippleHeight) {
    this.ripple?.dispose();
    this.rippleWidth = width;
    this.rippleHeight = height;
    this.ripple = Shekere.gpu.createFeedbackPass({
      name: "motion-ripple",
      width,
      height,
      format: "rgba16f",
      textures: ["motion"],
      uniforms: { decay: 0.97, intensity: 1.0 },
      build({ previous, textures, uniforms, uv }) {
        const texel = TSL.vec2(1 / width, 1 / height);
        const prior = previous.sample(uv).r;
        const horizontal = TSL.max(
          previous.sample(uv.add(TSL.vec2(texel.x, 0))).r,
          previous.sample(uv.sub(TSL.vec2(texel.x, 0))).r
        );
        const vertical = TSL.max(
          previous.sample(uv.add(TSL.vec2(0, texel.y))).r,
          previous.sample(uv.sub(TSL.vec2(0, texel.y))).r
        );
        const spread = TSL.max(
          prior,
          TSL.max(horizontal, vertical)
        ).mul(uniforms.decay);
        const seed = textures.motion.sample(uv).r.mul(uniforms.intensity);
        const next = TSL.max(spread, seed);
        return TSL.vec4(next, next, next, 1);
      }
    });

    const screenUv = TSL.screenUV.flipY();
    const cameraColor = Shekere.camera.textureNode.sample(screenUv).rgb;
    const ripple = this.ripple.node.sample(screenUv).r;
    this.scene.backgroundNode = cameraColor.add(TSL.vec3(0.1, 0.5, 1).mul(ripple));
  }

  this.ripple.update({
    // Host-owned stable nodes are read-only inputs. Passing the node avoids
    // rebinding when ADR 0008 swaps its internal motion render targets.
    textures: { motion: Shekere.camera.motion.maskNode },
    uniforms: {
      decay: 0.95 + audio.mid * 0.04,
      intensity: 1.0 + audio.bass * 3.0
    }
  });
}

export function cleanup(scene) {
  scene.backgroundNode = this.previousBackgroundNode;
  this.ripple?.dispose();
}
```

The API is not specific to cameras, motion, particles, or any visual shape.
Texture inputs may include camera video, ADR 0008 host-owned stable motion
nodes, raw textures, loaded assets, or another feedback pass. Uniform values
may be derived from audio, waveform summaries, MIDI, OSC, time, or arbitrary
sketch logic.

### 2. Build TSL graphs once and update only inputs

`options.build` will execute once when the pass is created. It returns the TSL
node used to calculate the next state. Named texture inputs will be represented
by stable texture-node wrappers initialized with a host-owned black fallback
texture. Named uniforms, `time`, and `deltaTime` will be stable TSL uniform
nodes.

`FeedbackPass.update()` will validate and update the values of those existing
nodes; it will not rebuild or recompile the TSL graph. Missing or `null`
texture inputs will use the black fallback. Unknown texture or uniform names,
wrong uniform dimensions, and non-finite values will reject that update and
report a sketch-scoped error without corrupting the pass's previous values.

When an input is a host-owned stable `TSL.TextureNode`, such as
`Shekere.camera.textureNode` or `Shekere.camera.motion.maskNode`, the feedback
service will resolve its latest underlying texture immediately before pass
execution. It will never mutate or dispose the source node. A feedback-pass
dependency must be supplied as the `FeedbackPass` object, not as
`FeedbackPass.node`, so the host can validate creation order and cycles.

Texture nodes in `FeedbackBuildContext` are not implicitly sampled. Build
callbacks must use `.sample(uv)` (and select `.r`, `.rgb`, or `.rgba` as
appropriate). The supplied `uv` is the feedback target's normalized
offscreen-pass UV. Displaying the result through `scene.backgroundNode` must
instead use `TSL.screenUV.flipY()` so sphere/background geometry UVs cannot
zoom or distort the texture.

When a texture input is another `FeedbackPass`, the host will retain that
pass-level dependency rather than snapshotting its current raw texture. Just
before executing the consumer, the host will point the consumer's stable input
node at the producer's latest output from the same sketch frame.

The `previous` node samples the pass's previous state. The sketch must not
provide, replace, or dispose the previous-state texture.

### 3. Schedule passes through the host render loop

Calling `FeedbackPass.update()` queues that pass for one execution. Multiple
calls for the same pass in one sketch frame will coalesce, with the last valid
input values winning. A pass that is not queued retains its previous state and
does not consume an offscreen render pass that frame.

The Visualizer frame order will be:

1. update stable camera bindings;
2. execute ADR 0008 camera motion analysis for a new camera frame;
3. call the active sketch's `update(context)`;
4. execute queued feedback passes;
5. render the main scene and post-processing pipeline.

This ordering lets a feedback pass consume the latest ADR 0008 motion state
and lets the main scene sample newly produced feedback in the same displayed
frame.

Passes will execute in creation order. A pass may sample its own prior state
through `previous`, or a pass created earlier through that pass's stable
`node`. Sampling a later-created pass creates a dependency cycle or stale
ordering and will be rejected during graph validation. The first
implementation will not reorder arbitrary dependency graphs.

The host will cap `deltaTime` at 0.1 seconds after stalls and will provide the
same monotonic Visualizer time used by sketch updates.

### 4. Keep the public sampling node stable

Each pass will own two render targets and alternate their read/write roles.
The pass will never sample from the render target currently being written.

`FeedbackPass.node` will keep stable identity. After each execution the host
will update its underlying texture value to the latest read target, allowing
TSL materials to retain the node across ping-pong swaps.

`FeedbackPass.texture` will return the current output texture and may therefore
return a different texture after an execution. It returns `null` after manual
disposal or pass failure. It is intended for inspection or APIs that explicitly
refresh their texture reference. TSL sketches should prefer the stable `node`
property.

Both render targets, their textures, the stable texture node, the fullscreen
quad, and the node material are host-owned. Sketches must not dispose or mutate
them except through `clear()` and `dispose()`.

`dispose()` is idempotent, cancels queued work and dependencies, disposes both
targets exactly once, changes `texture` to `null`, and points the still-stable
public `node` at the host-owned black fallback. Calls to `update()` or `clear()`
after disposal are rejected as sketch-scoped errors without affecting other
passes.

### 5. Validate size, format, and resource budgets

The initial implementation will enforce:

- integer width and height from 1 through 1024;
- at most 8 live feedback passes per sketch;
- at most 2,097,152 logical feedback pixels across a sketch, calculated as
  the sum of `width * height` for each pass before ping-pong duplication;
- `rgba8` by default;
- `rgba16f` only when supported by the active renderer backend;
- linear filtering, no mipmaps, and linear/no-color-space state textures;
- a four-component finite clear value, defaulting to transparent black.

Creation that exceeds a limit or requests an unsupported format will fail
before allocating partial resources. Limits are intentionally host policy and
may be revised by a future ADR or versioned configuration; sketches must not
assume unlimited GPU memory.

### 6. Preserve WebGPU and WebGL fallback compatibility

The initial API will use fullscreen TSL render passes and ordinary render
targets supported by the `WebGPURenderer` architecture and its WebGL 2
fallback. It will not expose WebGPU compute dispatch, storage buffers, atomics,
or backend-native handles.

Particle-like systems may encode position, velocity, lifetime, or other state
into feedback textures and sample that state from instanced TSL materials.
Native compute and storage-buffer simulations may be proposed separately if a
demonstrated effect cannot meet its performance target with texture feedback.

### 7. Centralize renderer state and lifecycle ownership

The GPU feedback service will use the same proven renderer-state boundary as
ADR 0008: `RendererUtils.saveRendererState()`,
`RendererUtils.resetRendererState()`, and
`RendererUtils.restoreRendererState()` in a `try/finally` block. The shared
pattern should be factored into a common host utility rather than reimplemented
with different state coverage. Feedback execution must not change the scene
camera, host post-processing nodes, or subsequent sketch rendering.

Every pass will belong to one sketch scope. On a code update, the host will:

1. call the old sketch's cleanup hook;
2. dispose the old scope in `finally`, even if cleanup throws;
3. open a candidate scope before importing the new module, so top-level or
   setup-time allocations cannot escape tracking;
4. commit that candidate as the active scope only after import and setup
   succeed;
5. roll back and dispose the entire candidate scope if import or setup fails.

Calling `dispose()` remains supported and expected in explicit sketch cleanup,
but the scope registry is the mandatory leak-prevention backstop. All remaining
active passes are also disposed on Visualizer unload. A sketch cannot transfer
a pass into another sketch scope.

`clear()` will queue a reset of both ping-pong targets to the configured clear
value at the next feedback execution phase, reset time-dependent pass history,
and retain allocated resources. If `clear()` and `update()` are called in the
same sketch frame, clearing occurs first and the queued update then reads the
clear value. Resize is not implicit: a sketch that needs another resolution
must dispose the pass and create a new one. Camera-motion-driven examples must
therefore create passes lazily from `camera.motion.width` and `height`, and
recreate them when either dimension changes.

If graph compilation or offscreen rendering fails, the host will disable and
dispose only the failing pass, replace its public node input with the black
fallback, set `texture` to `null`, and report the error. Other passes, the
active sketch, the camera, and the main render loop will continue.

## Expected Implementation Scope

A future implementation is expected to include, at minimum:

- a Visualizer-owned feedback service and sketch-scoped resource registry;
- validated TSL graph construction, named texture inputs, and scalar/vector
  uniform updates;
- ping-pong render targets, stable public texture nodes, queued execution, and
  deterministic renderer-state restoration;
- lifecycle integration with dynamic module loading, cleanup failures, hot
  reload, sketch switching, and Visualizer unload;
- examples for a motion-seeded expanding ripple and a texture-state particle
  system to demonstrate persistent state that ADR 0008's existing motion aura
  intentionally does not provide;
- English and Japanese API, ownership, limits, scheduling, and cleanup
  documentation;
- unit tests for validation/lifecycle and renderer integration tests for
  feedback correctness.

Implementation must proceed incrementally: first the scoped registry and
inactive API, then one ping-pong pass and scheduling, then dependencies and
limits, followed by examples and documentation.

## Consequences

### Positive

- Sketches can author stateful GPU effects without receiving the renderer or
  raw render targets.
- One API supports camera motion, audio-reactive feedback, ripples, smoke,
  accumulation, and texture-state particles without host-side effect classes.
- TSL graphs are compiled once while real-time inputs update stable nodes.
- Host scheduling prevents read/write hazards and restores renderer state.
- Sketch-scoped ownership provides a cleanup backstop for live-code failures.
- The same API can operate on WebGPU and the WebGL 2 fallback.

### Negative and Risks

- The API adds a second host-managed render graph before the main render and
  increases GPU memory and pass count.
- Creation-order dependencies are less flexible than a general graph
  scheduler and require sketches to construct passes deliberately.
- Texture-encoded simulations are less convenient than storage-buffer compute
  for some particle systems.
- User-authored TSL graphs can still fail compilation or be expensive within
  the enforced resource limits.
- Ping-pong output textures alternate, so non-TSL consumers must refresh raw
  texture references.

## Alternatives Considered

### Add effect-specific orb, particle, ripple, or fluid APIs

Rejected because each API would encode one visual style, duplicate lifecycle
logic, and constrain live-coded experimentation. These effects should be
examples built on the generic feedback primitive.

### Expose `WebGPURenderer` and render targets directly

Rejected because sketches could corrupt host state, create read/write hazards,
interfere with post-processing, and leak resources across reloads.

### Run feedback on Canvas or the CPU

Rejected for GPU-oriented image and simulation state because it introduces
pixel readback, CPU loops, uploads, and avoidable synchronization. CPU logic
remains appropriate for small scalar control data that does not require image
buffers.

### Add WebGPU compute and storage buffers immediately

Not selected for the initial version because it would make backend behavior
diverge from the WebGL fallback and substantially enlarge the public API.
Texture feedback covers the initial motion, aura, ripple, accumulation, and
particle-state use cases.

### Let each sketch implement private ping-pong targets

Rejected because the renderer is intentionally private and duplicated
implementations would repeat state restoration, error isolation, limits, and
cleanup logic.

## Verification Requirements for a Future Implementation

A future implementation must verify:

1. creating a valid pass allocates exactly two compatible render targets and
   exposes a stable node;
2. the first execution reads the configured clear value as `previous`;
3. consecutive executions read the prior output and never sample the active
   write target;
4. a pass that is not queued performs no offscreen render and retains state;
5. multiple updates in one frame coalesce with the last valid values;
6. named texture and uniform values update without rebuilding the TSL graph,
   and ADR 0008 stable texture nodes resolve their latest underlying texture
   immediately before execution;
7. missing textures use the black fallback and invalid updates preserve prior
   valid inputs;
8. passes execute in creation order, invalid forward/cyclic dependencies are
   rejected, and a feedback dependency passed as `FeedbackPass.node` is
   rejected in favor of the `FeedbackPass` object;
9. `node` identity remains stable while `texture` follows ping-pong output,
   and disposal or failure leaves the node on fallback with `texture === null`;
10. time is monotonic and `deltaTime` is capped at 0.1 seconds;
11. `clear()` resets both targets without reallocating them, and a same-frame
    update executes after the clear;
12. size, pass-count, logical-pixel, format, clear-value, and finite-uniform
    validation occurs before partial allocation or mutation;
13. the shared ADR 0008 `RendererUtils` state boundary leaves renderer target,
    viewport, scissor, clear, scene, and post-processing state unchanged after
    feedback execution;
14. manual dispose, candidate-scope commit and rollback, cleanup success,
    cleanup failure, module import/setup failure, hot reload, sketch switching,
    and Visualizer unload release all scoped GPU resources exactly once;
15. a failing pass is isolated and does not stop other passes or the main
    render loop;
16. the frame order is camera binding, ADR 0008 analysis, sketch update,
    feedback execution, then main/post-processing render;
17. a motion-seeded ripple uses actual `camera.motion.width` and `height`,
    recreates on dimension changes, and persists independently after the source
    motion stops;
18. texture-state particle examples demonstrate that the API is not specific
    to camera motion;
19. the supported WebGPU and WebGL 2 paths produce equivalent feedback
    semantics within expected numeric tolerance;
20. no public renderer, raw render target, backend handle, CPU image readback,
    or frame-sized Tauri IPC path is introduced;
21. feedback build nodes sample normalized pass `uv`, while any
    `scene.backgroundNode` presentation uses `TSL.screenUV.flipY()` without
    zoom, distortion, or vertical inversion;
22. TypeScript strict checks, production build, documentation build, and
    packaged application tests pass.
