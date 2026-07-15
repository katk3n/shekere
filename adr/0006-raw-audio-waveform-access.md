# 0006: Raw Audio Waveform Access

## Status

Proposed

## Date

2026-07-16

## Context

Shekere currently exposes frequency-domain and feature-extraction data to
sketches, including normalized volume, bass, mid, high, logarithmic FFT bands,
and Meyda features. This is sufficient for spectrum visualizers and semantic
audio reactivity, but it does not expose the time-domain shape of the incoming
audio signal.

Without time-domain samples, sketches cannot accurately implement visuals such
as:

- conventional oscilloscope traces;
- circular, radial, and three-dimensional waveform renderings;
- stereo X/Y and Lissajous figures;
- waveform history, ribbons, and terrain;
- geometry deformation based on individual audio samples;
- visual interactions that use a waveform as a boundary or surface.

The Control Panel also cannot show whether the selected input is clipped,
stereo channels are unbalanced, or one channel is silent. Existing volume and
frequency monitors do not communicate these conditions as clearly as a small
time-domain display.

ADR 0001 requires audio processing to remain in the TypeScript/Web Audio API
layer. Rust audio processing libraries are prohibited. The waveform feature
must preserve this boundary and must not introduce a second microphone capture
pipeline.

## Decision

### 1. Capture time-domain data in the Visualizer

The existing Visualizer audio pipeline will remain the sole owner of the
`MediaStream`, `AudioContext`, and microphone or audio-interface capture.

Time-domain data will be obtained from Web Audio API `AnalyserNode` instances
using `getFloatTimeDomainData()`. No Rust audio processing, separate
`getUserMedia()` call, `ScriptProcessorNode`, or `AudioWorklet` will be added
for this feature.

### 2. Provide mono and stereo waveform channels

The input will be connected to a `ChannelSplitterNode` so that left and right
channels can be analyzed independently. The existing mixed analysis path used
for FFT and Meyda extraction will remain available.

Sketches will receive the following API in `update(context)`:

```javascript
export function update({ audio }) {
  const mono = audio.waveform.mono;
  const left = audio.waveform.left;
  const right = audio.waveform.right;
}
```

The waveform contract will be:

```typescript
interface AudioWaveform {
  mono: Float32Array;
  left: Float32Array;
  right: Float32Array;
}
```

Each sample is a normalized floating-point amplitude, normally in the range
`-1.0` to `1.0`.

For a mono input, `mono` will contain the captured signal and `left` and
`right` will expose equivalent data. Sketches will therefore not need special
fallback logic for mono devices.

### 3. Use the existing FFT size as the waveform size

Each full-resolution waveform array will contain `FFT_SIZE` samples. With the
current configuration, this is 4096 samples per channel.

The arrays and their backing buffers will be allocated once and reused on
subsequent animation frames. The audio pipeline must not allocate three new
4096-sample arrays on every frame.

The full-resolution arrays are intended for sketches running in the Visualizer
window. A sketch may downsample them further when its visual geometry requires
fewer points.

### 4. Preserve the existing audio API

`audio.waveform` will be additive. The existing properties remain unchanged:

- `audio.volume`;
- `audio.bass`;
- `audio.mid`;
- `audio.high`;
- `audio.bands`;
- `audio.features`.

Existing sketches that do not use `audio.waveform` will continue to work
without modification.

When audio capture is inactive or unavailable, all waveform arrays will remain
available and contain zeroes. The API will not switch between an array and
`null` based on permission or device state.

### 5. Send only a reduced diagnostic waveform to the Control Panel

The full-resolution waveform arrays will not be sent over Tauri IPC. Sending
three 4096-sample arrays repeatedly would add unnecessary serialization,
transfer, allocation, and React rendering costs.

Instead, the Visualizer will generate a diagnostic preview for the left and
right channels:

- target resolution: 128 buckets per channel;
- contents: minimum and maximum amplitude for each bucket;
- update rate: at most 10 frames per second, using the existing audio activity
  synchronization cadence;
- transport: included in, or synchronized with, the existing
  `audio-activity` event;
- mono input: both displayed channels show equivalent data.

Min/max downsampling is selected instead of taking every Nth sample because it
preserves short transients and clipping peaks that could otherwise disappear
from the preview.

The Control Panel will eventually render this preview directly to a Canvas,
without storing or reconstructing the full 4096-sample waveform. It may also
show a clipping indicator when a channel approaches the normalized limits.

The Control Panel waveform is a diagnostic representation, not part of the
sketch API and not a source for further audio analysis.

### 6. Keep feature extraction in the existing mixed path

FFT bands and Meyda features will continue to be calculated from the existing
mixed analysis path. Introducing left and right waveform analyzers does not
require duplicating Meyda extraction for both channels.

Per-channel FFT or per-channel Meyda features are outside the scope of this
ADR. They may be considered separately if a concrete use case requires them.

## Expected Implementation Scope

This ADR does not implement the feature. A future implementation is expected
to modify, at minimum:

- `src/visualizer.ts` for waveform capture, channel splitting, buffer reuse,
  the sketch context, and preview downsampling;
- `src/App.tsx` for the Control Panel diagnostic waveform display;
- the English and Japanese audio and sketch API documentation;
- one or more example sketches demonstrating mono, stereo, and oscilloscope
  rendering.

The implementation must be delivered in small increments and verified after
each increment in accordance with the project workflow.

## Consequences

### Positive

- Sketches can implement genuine oscilloscope and phase-sensitive visuals.
- Stereo information becomes available independently from frequency-domain
  analysis.
- Three.js and TSL sketches can use audio samples for geometry and shader
  deformation.
- The Control Panel can diagnose clipping, silence, and channel imbalance.
- The design remains compliant with ADR 0001 by using Web Audio API only.
- Existing sketches retain full backward compatibility.

### Negative and Risks

- Three time-domain reads per animation frame add CPU work to the Visualizer.
- Additional `AnalyserNode` and `ChannelSplitterNode` instances increase the
  complexity of audio setup, teardown, and device switching.
- Some browsers or audio devices may present mono sources through a stereo
  channel layout. The implementation must verify actual channel behavior.
- An oscilloscope trace may appear horizontally unstable without zero-crossing
  or trigger alignment. Stabilization is a sketch-level concern for the first
  version and is not part of the host waveform contract.
- The Control Panel preview adds a small IPC payload and Canvas drawing cost,
  although the capped rate and reduced representation limit that overhead.

## Alternatives Considered

### Expose only a mono waveform

Rejected because it would prevent stereo X/Y, phase, balance, and independent
left/right visualizations. Stereo access is inexpensive to include while the
audio graph is being extended.

### Send full-resolution waveform data to the Control Panel

Rejected because the Control Panel only needs a diagnostic display. Full
resolution would increase IPC and UI costs without improving its purpose.

### Perform audio capture or waveform processing in Rust

Rejected because it violates ADR 0001 and would duplicate the existing Web
Audio API capture pipeline.

### Let each sketch call `getUserMedia()` independently

Rejected because it duplicates microphone access, complicates permissions and
device selection, risks exclusive-access conflicts, and bypasses the shared
audio configuration.

### Use an `AudioWorklet`

Not selected for the initial implementation. `AnalyserNode` already provides
the required time-domain samples with substantially less lifecycle and message
passing complexity. An `AudioWorklet` may be reconsidered if later profiling
shows that sample-accurate processing outside the render loop is required.

## Verification Requirements for a Future Implementation

A future implementation must verify:

1. existing FFT, Meyda, device selection, and sensitivity behavior remains
   functional;
2. the waveform arrays are stable, reused, and contain normalized samples;
3. mono devices provide safe equivalent left and right data;
4. stereo devices produce independently observable left and right data;
5. starting, stopping, and changing the audio device correctly rebuilds and
   disposes the waveform audio graph;
6. denying microphone permission leaves zero-filled waveform arrays and does
   not crash the render loop;
7. the Control Panel preview never receives full-resolution buffers and is
   throttled to the intended rate;
8. min/max preview downsampling preserves short peaks and clipping events;
9. TypeScript strict-mode checks and the production build pass;
10. an oscilloscope example renders continuously without material frame-rate
    regression.
