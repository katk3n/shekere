# Shekere

<p align="center">
  <img src="shekere-logo.png" width="400" alt="shekere logo">
</p>

**Shekere** is a live-coding environment for creating interactive audio-visual art with JavaScript and [Three.js](https://threejs.org/).

Whether you're performing live or sketching new visual concepts, Shekere provides a seamless bridge between sound analysis and 3D graphics. Write your sketch in any text editor, and see the results instantly.

---

## 🚀 Getting Started

### 1. Download & Launch
Download the latest version of Shekere for macOS from the [GitHub Releases](https://github.com/katk3n/shekere/releases) page.
- Open the `.dmg` file and drag Shekere to your Applications folder.
- **First Launch**: Since the app is currently unsigned, macOS will block it by default. To open it:
  1. Open the app and click **OK** on the warning dialog.
  2. Go to **System Settings** > **Privacy & Security**.
  3. Scroll down to the **Security** section and click **"Open Anyway"** for Shekere.

### 2. Prepare a Sketch File
Create a new `.js` file anywhere on your computer. Here is a minimal "Hello World" template:

```js
// my_art.js

export function setup(scene) {
  // One-time setup: Add objects to the scene
  const geometry = new THREE.SphereGeometry(1, 32, 32);
  const material = new THREE.MeshNormalMaterial();
  this.sphere = new THREE.Mesh(geometry, material);
  scene.add(this.sphere);
}

export function update({ time, audio }) {
  // Every frame: Animate based on time or audio
  this.sphere.position.y = Math.sin(time) * 2;
  
  // React to bass volume
  const s = 1 + audio.bass;
  this.sphere.scale.set(s, s, s);
}

export function cleanup(scene) {
  // Cleanup: Remove objects when switching sketches
  scene.remove(this.sphere);
  this.sphere.geometry.dispose();
  this.sphere.material.dispose();
}
```

### 3. Load & Live-Edit
1. Launch Shekere. Two windows will appear: **Control Panel** and **Visualizer**.
2. In the **Control Panel**, click **"Select JS File"** and choose your `.js` file.
3. Click **"Enable Mic"** to start the audio analysis.
4. Open the `.js` file in your favorite text editor (e.g., VS Code). Every time you **save** the file, the Visualizer will hot-reload your changes instantly!

---

## 🎨 Sketch API Reference

### Lifecycle Functions
Export these functions to define your sketch behavior:

| Function | When called | Argument |
|---|---|---|
| `setup(scene)` | Once when the file is loaded | `scene` — Three.js `Scene` object |
| `update(context)` | Every frame (~60fps) | `context` — Data object (see below) |
| `cleanup(scene)` | Just before the sketch is replaced | `scene` — Three.js `Scene` object |

### The `context` Object
The `update` function receives a context object containing time and real-time audio data:

```js
export function update({ time, audio, midi }) {
  // time  : elapsed seconds since the app started (number)
  // audio : real-time microphone analysis (see below)
  // midi  : real-time MIDI input (see below)
}
```

### `audio` Data
Shekere analyzes your microphone input and provides categorized frequency data:

| Property | Type | Description |
|---|---|---|
| `audio.volume` | `0.0 – 1.0` | Overall loudness |
| `audio.bass` | `0.0 – 1.0` | Low-frequency energy (27.5 – 250 Hz) |
| `audio.mid` | `0.0 – 1.0` | Mid-frequency energy (250 Hz – 2 kHz) |
| `audio.high` | `0.0 – 1.0` | High-frequency energy (2 – 4.2 kHz) |
| `audio.bands` | `Array(256)` | Full spectrum (256 linear bands from 27.5 Hz to 4.18 kHz) |

### `midi` Data
Shekere automatically connects to all available MIDI input devices. MIDI values are normalized from `0–127` to `0.0–1.0`.

| Property | Type | Description |
|---|---|---|
| `midi.notes` | `Array(128)` | Velocity of currently pressed notes (index 0-127) |
| `midi.cc` | `Array(128)` | Values of Control Change messages (knobs, sliders, etc.) |

Example: `const volume = midi.cc[7];` or `if (midi.notes[60] > 0) { ... }`

### Three.js Integration
The `THREE` library is globally available in your sketches—no imports required. Simply use `new THREE.BoxGeometry(...)`, `new THREE.MeshStandardMaterial(...)`, etc.

---

## 💡 Pro Tips
- **Performance**: While Shekere handles reloading, always implement the `cleanup()` function to dispose of geometries and materials. This prevents memory usage from creeping up during long sessions.
- **Lighting**: By default, the scene is dark. Use `THREE.AmbientLight` or `THREE.PointLight` in your `setup()` to illuminate your objects, or use `MeshNormalMaterial` for a quick unlit look.
- **Spectrum**: Use `audio.bands` to create detailed frequency visualizers. Each index in the 256-length array corresponds to a specific pitch from low (index 0) to high (index 255).

---

## 📜 Examples
Check the `examples/` directory in this repository for reference scripts:
- `audio_reactive_knot.js`: A simple reactivity demo.
- `spectrum.js`: A high-resolution 256-band spectrum bars implementation.
