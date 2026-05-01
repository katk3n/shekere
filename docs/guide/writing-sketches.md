# Writing Sketches

Shekere allows you to write visuals using JavaScript and **Three.js**. This guide explains the Lifecycle API and how to interact with the visualizer's data.

## Lifecycle API

Every Shekere sketch is a JavaScript module that exports specific lifecycle functions.

### 1. `setup(scene)`
Called once when the sketch is loaded. Use this to initialize your 3D objects, lights, and materials.
- **Argument**: `scene` (A `THREE.Scene` object).
- **Return**: An optional configuration object to set audio ranges or renderer properties.

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

## The `context` Object

The `update` function receives a rich object containing the following:

| Property | Type | Description |
| :--- | :--- | :--- |
| `time` | `number` | Total elapsed time in seconds. |
| `audio` | `object` | Processed audio data (volume, bass, mid, high, bands). |
| `midi` | `object` | MIDI input data (`midi.notes[0-127]`, `midi.cc[0-127]`). |
| `osc` | `object` | Latest OSC data per address (e.g., `osc['/play']`). |
| `bloom` | `object` | Control post-processing bloom (`strength`, `radius`, `threshold`). |
| `rgbShift` | `object` | Control RGB shift amount. |

## Audio Data Details

### Basic Properties (`audio`)
- `audio.volume`: Overall loudness (0.0 - 1.0).
- `audio.bass` / `mid` / `high`: Average intensity of specific frequency ranges.
- `audio.bands`: An array of 256 frequency bins (FFT data).

### Advanced Features (`audio.features`)
Shekere uses **Meyda.js** for deep audio analysis.
- `audio.features.rms`: Perceived loudness.
- `audio.features.zcr`: Zero-crossing rate (good for detecting percussive sounds).
- `audio.features.spectralCentroid`: Indicates how "bright" the sound is.

## The `Shekere` Global Object

Shekere provides global utilities to assist with development:

- `Shekere.clearScene(container)`: Safely disposes all objects and materials in a scene.
- `Shekere.SKETCH_DIR`: The absolute path to the current sketch's directory. Useful for loading local assets like textures.
- `THREE`: The entire Three.js library is available globally. No imports required.

## Post-Processing

You can control visual effects directly from your code. Changes are automatically synced with the Control Panel UI.

```javascript
export function update({ audio, bloom, rgbShift }) {
  bloom.strength = audio.bass * 3.0;
  rgbShift.amount = audio.high * 0.02;
}
```
