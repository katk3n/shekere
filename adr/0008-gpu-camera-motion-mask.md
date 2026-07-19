# 0008: GPU Camera Motion Mask and Trail

## Status

Implemented

## Date

2026-07-19

## Context

ADR 0007 exposes the Visualizer-owned camera stream to sketches as a
host-owned `THREE.VideoTexture`. This supports direct video mapping and
single-frame image effects, but it does not identify which parts of the image
have moved. Effects such as an aura following a moving arm require temporal
information: at minimum, the current camera frame must be compared with the
previous frame.

Reading full-resolution frames into `ImageData` every render frame would move
camera data from the GPU back to the CPU, allocate or update large buffers, and
can stall rendering. Sending motion data through Tauri IPC would add the same
serialization and transport costs rejected by ADR 0007. The application
already uses `WebGPURenderer` and TSL under ADR 0005, and Three.js provides the
render-target and fullscreen-node primitives needed to keep temporal image
processing on the GPU.

This ADR covers motion-based visual effects, not semantic body understanding.
It must support effects that follow any moving image region while clearly
distinguishing that capability from arm keypoint tracking, pose estimation,
or person segmentation.

Motion detection and effect generation are separate responsibilities. This
ADR produces generic GPU textures that describe where motion exists. Sketches
may combine those textures directly with existing `audio`, `midi`, `osc`, and
`time` inputs. Effects that need their own evolving GPU state are covered by
ADR 0009 rather than by an effect-specific camera API.

## Decision

### 1. Analyze camera motion in the Visualizer on the GPU

The Visualizer will own a `CameraMotionAnalyzer` adjacent to the existing
camera lifecycle module. It will use Three.js `RenderTarget`, `QuadMesh`,
`NodeMaterial`, and TSL nodes to compare camera frames without calling
`getImageData()`, `readPixels()`, or a Rust/native image-processing API.

The analyzer will run before the active sketch's `update(context)` call so the
sketch sees the newest completed motion textures. Its offscreen passes must
save and restore the renderer's current render target and relevant render
state before the main scene and post-processing pipeline runs.

No camera frame, motion mask, or trail texture will cross Tauri IPC. The
Control Panel will not receive a motion preview in the initial implementation.

### 2. Make analysis opt-in per sketch

Motion analysis consumes GPU time and memory, so it will be disabled unless
the active sketch requests it from `setup(scene)`:

```javascript
export function setup(scene) {
  return {
    camera: {
      motion: {
        enabled: true,
        threshold: 0.08,
        blur: 6,
        decay: 0.94
      }
    }
  };
}
```

The configuration contract will be:

```typescript
interface CameraMotionConfig {
  enabled?: boolean;   // default: false
  threshold?: number;  // default: 0.08, clamped to 0.0-1.0
  blur?: number;       // default: 6, clamped to 0-20 analysis pixels
  decay?: number;      // default: 0.94, clamped to 0.0-0.999
}
```

Omitting `camera.motion` or setting `enabled: false` disables the analyzer for
that sketch. Sketch switching and hot reload will apply the new sketch's
configuration without stopping the camera stream.

### 3. Process at reduced resolution and camera frame rate

The longest analysis dimension will be 320 pixels. The other dimension will
be calculated from the actual camera aspect ratio, with a minimum of one
pixel. A 16:9 camera will therefore use 320x180 render targets. Analysis
textures will use linear filtering, no mipmaps, and linear/no-color-space data
semantics so the masks are not treated as display color.

Analysis will occur at most once for each new video frame. The analyzer will
track the underlying video's `currentTime` or equivalent new-frame signal and
will not repeat all offscreen passes when the Visualizer renders multiple
times for the same camera frame.

Changing camera device or capture dimensions replaces the `VideoTexture` and
will reinitialize all motion history at the new aspect ratio.

### 4. Generate a current mask and a decaying trail

For each new camera frame, the analyzer will execute the following GPU steps:

1. On the first frame, copy the current frame into the previous-frame target,
   clear mask and trail targets to black, and report inactive motion. This
   prevents a full-screen flash when analysis starts.
2. Sample the current and previous frames, convert both to luminance, and
   calculate the absolute difference.
3. Convert the difference to a soft mask using
   `smoothstep(threshold, threshold + 0.04, difference)`.
4. Apply a separable Gaussian blur using the configured radius to expand and
   soften moving regions for aura-style effects.
5. Update a ping-pong trail target using
   `max(blurredMask, previousTrail * decay)`.
6. Copy the current frame into the previous-frame target for the next camera
   frame.

Separate ping-pong render targets will be used wherever a pass needs to read
the previous value while writing the next value. A texture will never be
sampled while it is simultaneously attached as the active render target.

The mask represents movement in the newest frame. The trail represents recent
movement with temporal decay and is the preferred source for glowing aura,
afterimage, distortion-mask, geometry-mask, and preallocated-instance effects.

### 5. Expose stable, host-owned motion data to sketches

ADR 0007's `CameraData` will be extended additively with a stable `motion`
object:

```typescript
interface CameraMotionData {
  active: boolean;
  maskTexture: THREE.Texture | null;
  trailTexture: THREE.Texture | null;
  width: number;
  height: number;
}

interface CameraData {
  active: boolean;
  texture: THREE.VideoTexture | null;
  width: number;
  height: number;
  frameRate: number;
  motion: CameraMotionData;
}
```

For TSL graphs created during `setup(scene)`, the Visualizer will also expose
stable host-owned texture nodes through the global Shekere namespace:

```typescript
interface ShekereCameraMotionNodes {
  readonly maskNode: TSL.TextureNode;
  readonly trailNode: TSL.TextureNode;
}

Shekere.camera.motion: ShekereCameraMotionNodes;
```

Both nodes keep the same identity for the lifetime of the Visualizer. The host
updates their underlying texture values after analysis and ping-pong swaps.
While analysis is disabled, initializing, unavailable, or failed, both nodes
sample a host-owned black fallback texture. Sketches may sample these nodes but
must not replace their values or dispose the nodes or fallback texture. TSL
materials should prefer these stable nodes over manually rebinding raw texture
references every frame.

`camera.motion` will keep the same identity across render frames. When motion
analysis is disabled, initializing, unavailable, or failed, its contract is:

- `active` is `false`;
- `maskTexture` and `trailTexture` are `null`;
- `width` and `height` are `0`.

Restarting analysis, changing devices, or changing capture dimensions may
replace the texture references. Sketches must update TSL or material texture
references when either reference changes.

All motion textures are owned by the Shekere host. Sketch cleanup functions
must not dispose them. Sketches may sample them from TSL with patterns such as
`TSL.texture(camera.motion.trailTexture)`, but the host will not expose the
renderer or render targets themselves as public sketch API.

### 6. Combine motion with existing real-time inputs

ADR 0008 does not define an effect language or restrict motion textures to a
specific visual primitive. An ordinary sketch `update(context)` receives
camera motion data together with the existing input APIs:

```javascript
export function update({ camera, audio, midi, osc, time, bloom }) {
  const motionTrail = camera.motion.trailTexture;

  // The motion texture selects where the effect appears. Existing inputs can
  // control how it looks without another motion-analysis API.
  this.auraTextureNode.value = motionTrail;
  this.bassUniform.value = audio.bass;
  bloom.strength = 0.5 + audio.bass * 3.0;
}
```

With ADR 0008 and existing Three.js/TSL APIs, sketches can implement:

- motion-localized color, glow, reveal, distortion, and displacement;
- audio-reactive aura intensity, hue, blur, and geometry deformation;
- MIDI- or OSC-controlled thresholds in the visual material;
- preallocated instanced meshes whose visibility is selected by the mask;
- effects that use the host-provided decaying trail as their only history.

ADR 0008 alone does not provide independent state for newly generated visual
elements. Effects whose state must continue evolving after it leaves the
motion mask, such as particles with individual velocity and lifetime, growing
ripples, smoke, or iterative fluid-like feedback, will use the generic
host-managed GPU feedback facility defined by ADR 0009.

### 7. Centralize lifecycle and failure handling

The analyzer will dispose all render targets, node materials, fullscreen-pass
resources, and texture references when:

- motion analysis is disabled by the active sketch;
- the camera stops or fails;
- the camera texture or capture dimensions change;
- the selected device changes or disconnects;
- the Visualizer unloads.

Sketch reload or switching will not dispose resources still required by the
next sketch when the next sketch enables an equivalent motion configuration;
the analyzer may reuse compatible targets after resetting their temporal
history. It must never allow a sketch to retain ownership of those resources.

An analyzer initialization or rendering failure will set `camera.motion` to
its inactive contract, dispose partial resources, and log a useful error. It
will not stop the camera stream, sketch update loop, or main render pipeline.

## Expected Implementation Scope

A future implementation is expected to include, at minimum:

- a Visualizer-owned TypeScript camera motion analyzer with TSL offscreen
  passes and deterministic cleanup;
- sketch configuration parsing and `context.camera.motion` injection;
- stable `Shekere.camera.motion.maskNode` and `trailNode` bindings with a black
  fallback texture;
- lifecycle integration with camera start, stop, device switching, hot reload,
  and Visualizer unload;
- an audio-reactive aura example that samples `trailTexture` without disposing
  it, and does not depend on ADR 0009;
- English and Japanese camera-motion API documentation;
- automated lifecycle/configuration tests and GPU integration verification.

Implementation must be delivered incrementally. The analyzer lifecycle and
inactive API contract should be completed before the GPU passes, followed by
the sketch API/example and documentation.

## Consequences

### Positive

- Moving arms and other moving regions can drive aura, glow, afterimage, and
  particle effects without per-frame CPU pixel readback.
- Analysis remains local to the Visualizer and adds no frame-sized IPC data.
- Reduced-resolution processing and new-frame gating limit GPU cost.
- Sketches receive reusable motion textures while the host retains resource
  ownership and lifecycle control.
- Existing camera and non-camera sketches remain compatible because motion
  analysis is additive and opt-in.
- Motion textures remain generic inputs that can be combined with audio, MIDI,
  OSC, and time without an effect-specific host API.

### Negative and Risks

- Motion analysis adds several offscreen GPU passes and render-target
  allocations while enabled.
- Camera movement, exposure changes, lighting flicker, moving backgrounds, and
  video noise also appear as motion.
- A frame-difference mask does not know which pixels belong to an arm or even
  to a person.
- Reduced resolution produces soft spatial boundaries; this is intentional for
  aura effects but unsuitable for precise segmentation.
- WebGPU/WebGL backend and WebView differences require packaged-app testing.

## Alternatives Considered

### Read low-resolution frames with Canvas and `getImageData()`

Not selected for the initial implementation. It is simpler to prototype, but
still introduces GPU-to-CPU readback, CPU loops, and additional buffers. It
may remain a fallback only if a supported renderer backend cannot execute the
TSL render-target pipeline, and such a fallback must be designed separately
rather than activated silently.

### Read full-resolution frames on the CPU

Rejected because the bandwidth, allocation pressure, and render-thread stalls
conflict with real-time Visualizer performance and ADR 0007's decision not to
expose per-frame `ImageData`.

### Send frames or masks through Tauri IPC

Rejected because the producer and consumer both live in the Visualizer. IPC
would add serialization, copies, latency, and Control Panel work without
providing a rendering benefit.

### Expose the WebGPURenderer directly to sketches

Rejected because sketches could corrupt render-target state, dispose shared
resources, or interfere with the host post-processing pipeline. The host will
provide motion textures as a narrow capability instead.

### Add pose estimation or person segmentation

Deferred. MediaPipe, TensorFlow.js, ONNX Runtime, or equivalent models could
identify shoulders, elbows, wrists, or a person mask, but they introduce model
distribution, inference scheduling, backend selection, and separate accuracy
and privacy concerns. A future ADR may add semantic tracking after a concrete
use case demonstrates that generic motion masks are insufficient.

## Verification Requirements for a Future Implementation

A future implementation must verify:

1. motion analysis remains inactive and allocates no render targets unless a
   sketch opts in;
2. the first analyzed frame produces a black mask and trail rather than a
   full-screen motion flash;
3. an unchanged synthetic frame produces a near-zero mask;
4. a moving synthetic rectangle produces a localized mask and blurred trail;
5. the trail decays monotonically after motion stops;
6. threshold, blur, and decay values are defaulted and clamped as specified;
7. analysis runs no more than once for each new camera frame;
8. `camera.motion` keeps stable identity while texture references may change;
9. camera stop, failure, device change, dimension change, sketch disable, and
   Visualizer unload release all analyzer GPU resources;
10. sketch reload and switching reset temporal history without stopping the
    active camera stream;
11. sketches cannot dispose or mutate host render targets through the public
    API;
12. camera failure or analyzer failure leaves the main render loop running;
13. no `getImageData()`, GPU pixel readback, Rust image processing, or
    frame-sized Tauri IPC path is introduced;
14. the aura example reacts to moving image regions while static camera areas
    remain visually stable;
15. the aura example combines `trailTexture` with audio intensity using only
    ADR 0008 and existing sketch APIs;
16. stateful simulation remains outside this analyzer and is delegated to ADR
    0009 rather than implemented as an orb- or particle-specific API;
17. TypeScript strict checks, production build, documentation build, and
    packaged macOS camera testing pass.
