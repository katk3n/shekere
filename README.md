# Shekere

<p align="center">
  <img src="shekere-logo.png" width="400" alt="shekere logo">
</p>

**Shekere** is a live-coding environment for creating interactive audio-visual art with JavaScript and [Three.js](https://threejs.org/).

Write a JS file, select it in the Control Panel, and it renders instantly in the Visualizer. Every time you save the file, it hot-reloads automatically.

---

## Getting Started

### 1. Launch

```bash
npm install
npm run tauri dev
```

Two windows will open:
- **Control Panel** — file selection, mic toggle, and other controls
- **Visualizer** — the black canvas where your art renders

### 2. Create a sketch file

Create a `.js` file anywhere on your machine:

```js
// my_art.js

export function setup(scene) {
  // Called once on load. Add objects to the scene here.
  const geometry = new THREE.BoxGeometry(1, 1, 1);
  const material = new THREE.MeshNormalMaterial();
  this.cube = new THREE.Mesh(geometry, material);
  scene.add(this.cube);
}

export function update({ time, audio }) {
  // Called every frame. Drive animations here.
  this.cube.rotation.y = time;
}

export function cleanup(scene) {
  // Called when switching to another file.
  // Always remove and dispose objects to prevent memory leaks.
  scene.remove(this.cube);
  this.cube.geometry.dispose();
  this.cube.material.dispose();
}
```

### 3. Select the file in the Control Panel

Click **"Select JS File"** and choose your `.js` file. The Visualizer updates immediately. From then on, saving the file triggers an automatic hot-reload.

---

## Sketch API Reference

### Lifecycle Functions

| Function | When called | Argument |
|---|---|---|
| `setup(scene)` | Once on load | `scene` — Three.js `Scene` |
| `update(context)` | Every frame | `context` — see below |
| `cleanup(scene)` | Before switching files | `scene` — Three.js `Scene` |

All three functions are optional — only `export` what you need.

### The `context` Object (`update` argument)

```js
export function update({ time, audio }) {
  // time  : elapsed seconds since launch (number)
  // audio : microphone analysis data (see below)
}
```

### `context.audio`

Audio data derived from microphone input via the Web Audio API.

| Property | Type | Description |
|---|---|---|
| `audio.volume` | `number` (0–1) | Overall loudness |
| `audio.bass` | `number` (0–1) | Low-frequency energy (27.5–250 Hz) |
| `audio.mid` | `number` (0–1) | Mid-frequency energy (250 Hz–2 kHz) |
| `audio.high` | `number` (0–1) | High-frequency energy (2–4.2 kHz) |
| `audio.bands` | `number[256]` (each 0–1) | Full spectrum (27.5 Hz–4,186 Hz divided into 256 bands) |

> **To use audio data**: click **"Enable Mic"** in the Control Panel. The OS will prompt for microphone permission.

### Three.js

`THREE` is available as a global — no import needed.

```js
const geo = new THREE.SphereGeometry(1, 32, 32);
const mat = new THREE.MeshStandardMaterial({ color: 0xff6600 });
```

---

## Hot Reload

Every time you **save** your file, the Visualizer automatically:

1. Calls `cleanup()` to tear down the current scene
2. Imports your updated code and calls `setup()` again

> **Important**: if you forget to call `scene.remove()` and `.dispose()` in `cleanup()`, memory will grow with every reload. Always clean up everything you create in `setup()`.

---

## Examples

| File | Description |
|---|---|
| `examples/audio_reactive_knot.js` | Audio-reactive TorusKnot — scales with bass, goes wireframe on highs |
| `examples/spectrum.js` | 256-band audio spectrum visualizer with a blue-to-red gradient |

See the comment block at the top of each file for usage details.

---

## Project Structure

```
shekere/
├── examples/                        # Example sketches
│   ├── my_sketch.js
│   └── spectrum.js
├── src/
│   ├── hooks/useAudioAnalyzer.ts    # Mic input & FFT analysis
│   ├── visualizer.ts                # Three.js host environment
│   └── App.tsx                      # Control Panel UI
└── adr/                             # Architecture Decision Records
```

---

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop shell | [Tauri v2](https://tauri.app/) (Rust) |
| Control Panel | React + Tailwind CSS |
| Visualizer | Vanilla TypeScript + Three.js |
| Audio analysis | Web Audio API (`AnalyserNode`, fftSize = 4096) |
| IPC | Tauri Events |
