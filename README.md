# Shekere

<p align="center">
  <img src="src/assets/shekere-logo.png" width="400" alt="Shekere logo">
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

### 3. Playlist & Sketch Switching
Shekere allowing you to manage multiple sketches using a TOML playlist file. This is ideal for live performances where you need to switch between different visual concepts quickly.

#### Create a Playlist File (`.toml`)
Create a `.toml` file and define your sketches. You can also map MIDI notes for navigation or direct slot jumping.

```toml
[midi.navigation]
next_note = 38 # Trigger "Next" sketch
prev_note = 36 # Trigger "Prev" sketch

[[sketch]]
file = "shader_stars.js" # Path relative to the TOML file
midi_note = 48         # Direct jump to this slot (C2)

[[sketch]]
file = "audio_bars.js"
midi_note = 49         # Direct jump to this slot (C#2)
```

### 4. Load & Live-Edit
1. Launch Shekere. Two windows will appear: **Control Panel** and **Visualizer**.
2. **Standard Load**: In the **Control Panel**, click the "File" icon next to a playlist slot to select a single `.js` file.
3. **Playlist Load**: Click **"Load Playlist"** and select your `.toml` file to load multiple sketches at once.
4. Click **"Enable Mic"** to start the audio analysis.
5. Every time you **save** an active `.js` file, the Visualizer will hot-reload your changes instantly!

---

## ⌨️ Controls & Shortcuts

| Action | Shortcut |
|---|---|
| **Jump to Slot 1-9** | `1` – `9` keys |
| **Next Sketch** | `→` (Right Arrow) or MIDI `next_note` |
| **Previous Sketch** | `←` (Left Arrow) or MIDI `prev_note` |
| **Direct Slot Jump** | Specific MIDI `midi_note` or `midi_cc` |

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
The `setup()` function can return an optional configuration object:

```js
export function setup(scene) {
  return {
    audio: {
      minFreqHz: 80,   // Lowest frequency to analyze
      maxFreqHz: 2000  // Highest frequency to analyze
    }
  };
}
```

### The `context` Object
The `update` function receives real-time data and effect controls:

```js
export function update({ time, audio, midi, osc, oscEvents, bloom, rgbShift, film, vignette }) {
  // time  : elapsed seconds (number)
  // audio : microphone FFT analysis
  // midi  : MIDI notes and CC values
  // osc   : Latest OSC data per address
  // bloom, rgbShift, film, vignette : Post-processing controls
}
```

---

## ✨ Post-Processing API

Shekere includes a powerful post-processing pipeline. You can control these effects via the **Control Panel UI** or directly from your **Sketch Code**. Both stay in sync automatically (**Bidirectional Sync**).

### 🌸 Bloom (Glow)
| Property | Range | Default | Description |
|---|---|---|---|
| `bloom.strength` | 0.0 – 3.0 | `0` | Overall intensity of the glow. |
| `bloom.radius` | 0.0 – 1.0 | `0` | Blur radius of the bloom. |
| `bloom.threshold` | 0.0 – 1.0 | `1.0` | Brightness threshold for blooming. |

### 🌈 RGB Shift
| Property | Range | Default | Description |
|---|---|---|---|
| `rgbShift.amount` | 0.0 – 0.05 | `0` | Color channel offset amount. |

### 🎞️ Film Grain
| Property | Range | Default | Description |
|---|---|---|---|
| `film.intensity` | 0.0 – 2.0 | `0` | Intensity of the noise grain. |

### 🎭 Vignette
| Property | Range | Default | Description |
|---|---|---|---|
| `vignette.offset` | 0.0 – 3.0 | `0` | Radius of the vignette. |
| `vignette.darkness` | 0.0 – 3.0 | `1.0` | Intensity/Blackness of the edges. |

---

## 🔊 Audio Data
The audio analyzer uses logarithmic frequency scaling to match human hearing.

#### Data Properties (`context.audio`)
| Property | Range | Description |
|---|---|---|
| `volume` | 0.0 – 1.0 | Root-mean-square average of all 256 bands. |
| `bass` | 0.0 – 1.0 | Average intensity (minFreqHz – 250 Hz). |
| `mid` | 0.0 – 1.0 | Average intensity (250 Hz – 2000 Hz). |
| `high` | 0.0 – 1.0 | Average intensity (2000 Hz – maxFreqHz). |
| `bands` | 0.0 – 1.0 | Array(256) of logarithmic frequency bins. |

---

## ⌨️ MIDI, OSC & More

- **MIDI**: Access `midi.notes[0-127]` and `midi.cc[0-127]` (all normalized 0.0 – 1.0).
- **OSC**: Port `2020`. Use `osc['/address']` for state or `oscEvents` for triggers.
- **Three.js**: The library is globally available as `THREE`. No imports required.

---

## 📜 Examples
Check the `examples/` directory for reference scripts covering Audio, MIDI, OSC, and Post-Processing effects. Each example includes comments explaining its specific mappings and logic.

---

## 💡 Pro Tips
- **Performance**: Always implement `cleanup()` to avoid memory leaks.
- **Auto-Sync**: If you change an effect in code (e.g., `bloom.strength = 1.5`), the Control Panel slider will automatically move to match!
- **Hot Reload**: Keep your editor and the Visualizer side-by-side for the best experience.
