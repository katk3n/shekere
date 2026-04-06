# Shekere

<p align="center">
  <img src="shekere-logo.png" width="400" alt="shekere logo">
</p>

**Shekere** is a live-coding environment for creating interactive audio-visual art with JavaScript and [Three.js](https://threejs.org/).

Whether you're performing live or sketching new visual concepts, Shekere provides a seamless bridge between sound analysis, MIDI, OSC, and 3D graphics. Write your sketch in any text editor, and see the results instantly.

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
export function setup(scene) {
  const geometry = new THREE.IcosahedronGeometry(1, 2);
  const material = new THREE.MeshNormalMaterial({ wireframe: true });
  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

export function update({ time, audio }) {
  this.mesh.rotation.y = time * 0.5;
  const s = 1 + audio.bass;
  this.mesh.scale.set(s, s, s);
}

export function cleanup(scene) {
  scene.remove(this.mesh);
  this.mesh.geometry.dispose();
  this.mesh.material.dispose();
}
```

### 3. Load & Live-Edit
1. Launch Shekere. Two windows will appear: **Control Panel** and **Visualizer**.
2. In the **Control Panel**, click **"Select JS File"** and choose your `.js` file.
3. Click **"Enable Mic"** to start the audio analysis.
4. Open the `.js` file in your favorite text editor. Every time you **save** the file, the Visualizer will hot-reload your changes instantly!

---

## 🎨 Sketch API Reference

### Lifecycle Functions
Export these functions to define your sketch behavior:

| Function | When called | Argument | Return Value |
|---|---|---|---|
| `setup(scene)` | Once when the file is loaded | `scene` (Three.js `Scene`) | `config` object (Optional) |
| `update(context)` | Every frame (~60fps) | `context` — Data object | `void` |
| `cleanup(scene)` | Just before the sketch is replaced | `scene` (Three.js `Scene`) | `void` |

### Sketch Configuration
The `setup()` function can return an optional configuration object to tune the analysis engine for your specific sketch:

```js
export function setup(scene) {
  // ... setup your meshes
  
  return {
    audio: {
      minFreqHz: 80,   // Lowest frequency to analyze
      maxFreqHz: 2000  // Highest frequency to analyze
    }
  };
}
```

#### Configuration Options (`config.audio`)
| Property | Type | Default | Description |
|---|---|---|---|
| `minFreqHz` | `number` | `27.5` | The lowest frequency (A0) mapped to `bands[0]`. |
| `maxFreqHz` | `number` | `4186.0` | The highest frequency (C8) mapped to `bands[255]`. |

### The `context` Object
The `update` function receives real-time data on every frame:

```js
export function update({ time, audio, midi, osc, oscEvents }) {
  // time  : elapsed seconds (number)
  // audio : microphone FFT analysis
  // midi  : MIDI notes and CC values
  // osc   : Latest OSC data per address
  // oscEvents : List of OSC messages received in the current frame
}
```

### 🔊 Audio Data
The audio analyzer uses logarithmic frequency scaling to match human hearing and musical octaves.

#### Data Properties (`context.audio`)
| Property | Type | Range | Description |
|---|---|---|---|
| `volume` | `number` | 0.0 – 1.0 | Root-mean-square average of all 256 bands. |
| `bass` | `number` | 0.0 – 1.0 | Average intensity from `minFreqHz` to **250 Hz**. |
| `mid` | `number` | 0.0 – 1.0 | Average intensity from **250 Hz** to **2000 Hz**. |
| `high` | `number` | 0.0 – 1.0 | Average intensity from **2000 Hz** to `maxFreqHz`. |
| `bands` | `number[]` | 0.0 – 1.0 | Array of 256 intensities for logarithmic frequency bins. |

**Processing Details**:
- **Tilt EQ**: Higher frequencies receive a linear gain boost (1.0x to 1.8x) to compensate for natural energy drop-off.
- **Power Scaling**: A 1.5x power curve is applied to increase visual contrast and suppress noise.

### ⌨️ MIDI Data
- `midi.notes`: Array(128) of velocity (0.0 – 1.0).
- `midi.cc`: Array(128) of control change values (0.0 – 1.0).

### 📡 OSC Data
Shekere listens for OSC messages on UDP port **2020**.
- `osc`: A dictionary of the latest data for each address (e.g., `osc['/dirt/play'].s`).
- `oscEvents`: A list of `{ address, data }` for messages that arrived *this frame*, ideal for triggering one-shot effects.

---

## 📜 Examples
Check the `examples/` directory for reference scripts covering Audio, MIDI, and OSC reactivity. Detailed usage and mapping info can be found in the comments within each example file.

---

## 💡 Pro Tips
- **Performance**: Always implement `cleanup()` to dispose of geometries and materials to avoid memory leaks.
- **Three.js**: The library is globally available as `THREE`. No imports required.
- **Hot Reload**: Keep your editor and the Visualizer side-by-side for the best live-coding experience.
