# 0007: Real-Time Camera Input

## Status

Implemented

## Date

2026-07-19

## Context

Shekere currently provides user sketches with real-time audio, MIDI, and OSC
inputs, but it does not provide access to live camera video. Camera input is a
common source for audiovisual and VJ sketches, including video mapped onto
geometry, feedback-style compositions, color and distortion effects, and
TSL-based image processing.

The Visualizer window owns the Three.js renderer and executes user sketch
`update(context)` functions. Sending camera frames from the Control Panel to
the Visualizer over Tauri IPC would require repeated serialization or image
encoding, add latency, and duplicate work. Allowing each sketch to acquire its
own camera stream would also duplicate permission and device lifecycle logic
and could leave camera tracks active after sketch reloads.

ADR 0001 requires functionality achievable with Web APIs to remain in the
TypeScript/WebView layer and keeps Rust responsibilities minimal. ADR 0005
establishes `WebGPURenderer` and TSL as the rendering architecture. Camera
input must preserve both decisions and remain compatible with the existing
multi-window design.

## Decision

### 1. Capture video in the Visualizer

The Visualizer will be the sole owner of the camera `MediaStream`. It will use
`navigator.mediaDevices.getUserMedia()` with video enabled and audio disabled.

The initial requested capture quality will be:

- width: 1280 pixels (`ideal`);
- height: 720 pixels (`ideal`);
- frame rate: 30 frames per second (`ideal`).

These values are preferences rather than exact requirements so that cameras
with different capabilities can still be used. The actual width, height, and
frame rate reported by the selected video track will be exposed to sketches.

Camera capture, frame handling, and texture creation will remain entirely in
TypeScript. No Rust camera library, native frame processing, or per-frame
Tauri IPC transport will be introduced.

### 2. Keep device controls in the Control Panel

The Control Panel will provide:

- a list of available video input devices;
- explicit camera start and stop controls;
- selection of the active camera;
- capture state and actionable error feedback.

Camera capture will never start automatically. It must begin in response to an
explicit user action so that permission prompts and privacy expectations are
clear.

The Control Panel will send only lifecycle and device-selection commands to
the Visualizer. The Visualizer will return device metadata and lightweight
status events. Raw frames, encoded frames, and the camera texture will not be
sent over IPC.

The existing Control Panel preview already represents the final Visualizer
render output. A separate camera-only preview will not be added. When a sketch
uses the camera texture, the composited result will naturally appear in the
existing preview.

### 3. Expose a host-owned Three.js VideoTexture

The Visualizer will create an `HTMLVideoElement` for the active stream and use
it as the source of a `THREE.VideoTexture`. Because Shekere uses
`WebGPURenderer`, the texture color space will be set to
`THREE.SRGBColorSpace`.

Sketches will receive the following additive API in `update(context)`:

```typescript
interface CameraData {
  active: boolean;
  texture: THREE.VideoTexture | null;
  width: number;
  height: number;
  frameRate: number;
}
```

Example usage:

```javascript
export function update({ camera }) {
  if (this.material.map !== camera.texture) {
    this.material.map = camera.texture;
    this.material.needsUpdate = true;
  }
}
```

The `camera` object will keep a stable identity across render frames. When the
camera is inactive, unavailable, or has failed, its contract will be:

- `active` is `false`;
- `texture` is `null`;
- `width`, `height`, and `frameRate` are `0`.

Restarting capture or changing devices may replace the `VideoTexture` because
the dimensions of a texture cannot safely be changed after first use. Sketches
must therefore update material texture references when `camera.texture`
changes.

The texture is owned by the Shekere host. Sketch cleanup functions must not
dispose it. This prevents one sketch from invalidating the shared camera input
or breaking the next sketch after hot reload or playlist switching.

### 4. Centralize lifecycle and resource cleanup

Camera lifecycle management will be separated from sketch lifecycle. Switching
or reloading a sketch will not stop an active camera stream.

The host will stop all tracks, clear the video element source, and dispose the
old `VideoTexture` when:

- the user stops the camera;
- the active device changes;
- the selected device is disconnected;
- a start request is superseded by a newer request;
- the Visualizer is unloaded.

Asynchronous start and device-switch operations must reject stale results. If
an older `getUserMedia()` request completes after a newer request, its tracks
must be stopped immediately rather than replacing the current stream.

Changing to a specifically selected device will use its device ID. If that
device cannot be opened, the host will report the failure instead of silently
switching to another camera.

### 5. Handle permissions and failures safely

The implementation will distinguish at least the following conditions for
Control Panel feedback:

- camera permission denied;
- no camera device available;
- selected device unavailable or disconnected;
- requested constraints unsupported;
- media capture APIs unavailable;
- unexpected capture or playback failure.

A failure must leave the sketch API in the inactive state and must not stop the
Visualizer render loop.

On macOS, a future implementation must add:

- `NSCameraUsageDescription` to `Info.plist`;
- `com.apple.security.device.camera` to the application entitlements.

Existing microphone permission and audio-input entitlements will remain
unchanged. Camera capture will request video only and will not replace or
combine with the existing microphone capture pipeline.

## Expected Implementation Scope

This ADR does not implement the feature. A future implementation is expected
to include, at minimum:

- a TypeScript camera lifecycle module owned by the Visualizer;
- Visualizer event handlers and `context.camera` injection;
- Control Panel state, device selection, start/stop controls, and errors;
- macOS camera usage description and sandbox entitlement changes;
- an example sketch using `THREE.VideoTexture`;
- English and Japanese camera and sketch API documentation.

The implementation must be delivered in small increments and verified after
each increment in accordance with the project workflow.

## Consequences

### Positive

- Sketches can use live camera video directly as a GPU texture with no
  per-frame IPC encoding or transfer.
- Camera capture remains compatible with the TypeScript-first boundary in ADR
  0001 and the WebGPU/TSL rendering pipeline in ADR 0005.
- Permission, device switching, and cleanup behavior are centralized rather
  than reimplemented by each sketch.
- Camera capture survives sketch hot reload and playlist switching.
- Existing sketches remain compatible because `context.camera` is additive.
- The existing final-output preview automatically includes camera-based
  compositions.

### Negative and Risks

- Live video adds camera decoding, GPU upload, and texture memory costs to the
  Visualizer render workload.
- Device labels may be unavailable before permission is granted, requiring
  generic labels in the initial device list.
- Actual resolution and frame rate can differ from the requested ideal values.
- Device switching replaces the texture reference, so sketches that cache a
  material map must follow the documented update pattern.
- Camera permissions and device behavior vary across operating systems and
  WebView implementations and require packaged-app verification.

## Alternatives Considered

### Capture in the Control Panel and send frames over IPC

Rejected because continuous frame serialization or encoding would add latency,
CPU cost, allocations, and IPC traffic. The frames are consumed by the
Visualizer, so capture belongs in that window.

### Let each sketch call `getUserMedia()`

Rejected because it would duplicate permission and lifecycle handling, make
device selection inconsistent, and increase the risk of leaked tracks during
hot reload.

### Process camera frames in Rust

Rejected because browser media APIs already provide the required capture path
and native processing would violate the TypeScript-first boundary in ADR 0001.

### Expose full-resolution ImageData on every frame

Not selected for the initial version. Reading pixels back for CPU processing
can stall the rendering pipeline and allocate large buffers. A separately
designed, reduced-resolution, on-demand API may be considered later for motion,
color, or computer-vision analysis.

### Add recording and still-image capture

Not selected because recording, storage, codecs, and snapshot workflows have
different permission, UI, and lifecycle requirements. This ADR covers live
input as a rendering source only.

## Verification Requirements for a Future Implementation

A future implementation must verify:

1. camera permission can be granted and denied without crashing either window;
2. capture does not begin before an explicit user action;
3. the requested default is 720p at 30 fps and actual track settings are
   exposed to sketches;
4. a camera texture can be mapped to Three.js geometry with correct color in
   the WebGPU renderer;
5. starting, stopping, restarting, and changing devices update
   `context.camera` correctly;
6. stale asynchronous start requests cannot replace a newer active stream;
7. old tracks and textures are released after stop, switch, disconnect, and
   Visualizer unload;
8. sketch reload and playlist switching do not interrupt camera capture or
   cause sketches to dispose the host-owned texture;
9. a missing device, disconnected device, unsupported API, and capture failure
   produce safe inactive state and useful Control Panel feedback;
10. microphone analysis and camera input operate simultaneously without
    combining their media streams;
11. the existing Control Panel preview shows the final camera-composited sketch
    without a second camera-frame IPC path;
12. macOS packaged builds contain the camera usage description and camera
    entitlement;
13. TypeScript strict-mode checks, the production application build, and the
    documentation build pass.
