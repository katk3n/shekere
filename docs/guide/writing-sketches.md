# Writing Sketches

Shekere allows you to write visuals using JavaScript and **Three.js**. It acts as a wrapper around Three.js, managing the rendering loop, scene, and camera for you, so you can focus on the logic of your sketch.

## Lifecycle API

Instead of writing a standard Three.js loop, every Shekere sketch exports specific functions.

### 1. `setup(scene)`
Called once when the sketch is loaded. Use this to initialize your 3D objects, lights, and materials.
- **Argument**: `scene` (A `THREE.Scene` object).
- **Return**: An optional configuration object to set audio ranges, renderer
  properties, or opt-in camera motion analysis.

```javascript
export function setup(scene) {
  const geometry = new THREE.BoxGeometry(1, 1, 1);
  const material = new THREE.MeshBasicMaterial({ color: 0x00ff00 });
  this.cube = new THREE.Mesh(geometry, material);
  scene.add(this.cube);

  return {
    audio: { minFreqHz: 80, maxFreqHz: 2000 }
  };
}
```

### 2. `update(context)`
Called on every frame (~60 times per second). Use this to animate your scene.
- **Argument**: `context` (An object containing real-time data).

```javascript
export function update({ time, audio, bloom }) {
  // Rotate the cube over time
  this.cube.rotation.x = time;
  
  // Make the bloom glow react to volume
  bloom.strength = audio.volume * 2.0;
}
```

### 3. `cleanup(scene)`
Called just before the sketch is replaced or reloaded.
- **Important**: You must clean up your objects to prevent memory leaks. The easiest way is using the helper below.

```javascript
export function cleanup(scene) {
  Shekere.clearScene(scene);
}
```

`Shekere.clearScene(scene)` removes all descendants and disposes each unique
geometry and material once, including resources used by meshes, lines, points,
and sprites. It intentionally does not dispose textures or resources that are
not attached directly to scene objects because their ownership cannot be
inferred. Explicitly dispose sketch-owned textures, event listeners, and other
external resources in `cleanup`.

## The `context` Object

The `update` function receives a rich object containing the following:

| Property | Type | Description |
| :--- | :--- | :--- |
| `time` | `number` | Total elapsed time in seconds. |
| `camera` | `object` | Live camera state, host-owned `VideoTexture`, and optional [motion textures](./camera-motion.md). |
| `audio` | `object` | Processed audio data (volume, bands, features, waveform). |
| `midi` | `object` | MIDI input data (`midi.notes[0-127]`, `midi.cc[0-127]`). |
| `osc` | `object` | Latest OSC data per address (e.g., `osc['/play']`). |
| `bloom` | `object` | Control post-processing bloom (`strength`, `radius`, `threshold`). |
| `rgbShift` | `object` | Control RGB shift amount. |
| `film` | `object` | Control Film Grain (`intensity`). |
| `vignette` | `object` | Control Vignette (`offset`, `darkness`). |

## Audio Data Details

### Basic Properties (`audio`)
- `audio.volume`: Overall loudness (0.0 - 1.0).
- `audio.bass` / `mid` / `high`: Average intensity of specific frequency ranges.
- `audio.bands`: An array of 256 frequency bins (FFT data).
- `audio.waveform.mono`, `.left`, `.right`: Reused `Float32Array` time-domain
  buffers with 4096 normalized samples each. Mono inputs expose equivalent
  left/right data; inactive capture exposes zero-filled buffers.

### Advanced Features (`audio.features`)
Shekere uses **Meyda.js** for deep audio analysis.
- `audio.features.rms`: Perceived loudness.
- `audio.features.zcr`: Zero-crossing rate (good for detecting percussive sounds).
- `audio.features.spectralCentroid`: Indicates how "bright" the sound is.

## The `Shekere` Global Object

Shekere provides global utilities to assist with development:

- `Shekere.clearScene(container)`: Removes descendants and disposes unique
  scene geometry and materials without disposing textures.
- `Shekere.SKETCH_DIR`: The absolute path to the current sketch's directory. Useful for loading local assets like textures.
- `Shekere.camera.textureNode`: Stable, host-owned TSL node for the live camera
  image, with a black fallback while inactive.
- `Shekere.camera.motion.maskNode` / `trailNode`: Stable, host-owned TSL nodes
  for camera motion graphs created during `setup(scene)`.
- `Shekere.gpu.createFeedbackPass(options)`: Creates a host-managed, sketch-scoped
  [GPU feedback pass](./gpu-feedback.md) for persistent texture state.
- `THREE`: The entire Three.js library is available globally. No imports required.
- `TSL`: The Three.js Shading Language module is available globally for building shader nodes.

## Post-Processing

You can control visual effects directly from your code. Changes are automatically synced with the Control Panel UI.

```javascript
export function update({ audio, bloom, rgbShift }) {
  bloom.strength = audio.bass * 3.0;
  rgbShift.amount = audio.high * 0.02;
}
```
