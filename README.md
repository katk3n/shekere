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

### 🛡️ Persistent Permissions (macOS)
If you find that Shekere asks for Microphone or File permissions **every time** you launch it, this is because the binary downloaded from GitHub is "unsigned." macOS resets permissions for unsigned apps upon every restart as a security measure.

To fix this and make permissions permanent, you can "re-sign" the app locally on your Mac:

1. Move **Shekere.app** to your `/Applications` folder.
2. Open **Terminal** and run the following two commands:
   ```bash
   # 1. Clear the "Quarantine" flag
   xattr -cr /Applications/Shekere.app

   # 2. Re-sign the app with your own local identity
   codesign --force --deep --sign - /Applications/Shekere.app
   ```
3. Launch Shekere and grant permissions one last time. They will now be remembered.

> [!CAUTION]
> **Security Warning**: Re-signing a binary bypasses macOS Gatekeeper's origin checks for that specific file. Only perform this on versions of Shekere you have downloaded from the official repository or built yourself. Use at your own risk.

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
  // Simplest way to clear the scene and free memory
  Shekere.clearScene(scene);
}
```

### 3. Playlist & Sketch Switching
Shekere allowing you to manage multiple sketches using a TOML playlist file. This is ideal for live performances where you need to switch between different visual concepts quickly.

#### Create a Playlist File (`.toml`)
Create a `.toml` file and define your sketches. You can map MIDI notes or OSC messages (parsed key/value pairs like those from TidalCycles) for navigation or direct slot jumping.

```toml
[midi.navigation.next]
note = 38 # Trigger "Next" sketch via MIDI

[midi.navigation.prev]
note = 36 # Trigger "Prev" sketch via MIDI

[osc.navigation.next]
key = "s"
value = "bd" # Trigger "Next" sketch when OSC argument contains `s="bd"`

[osc.navigation.prev]
key = "s"
value = "cp" # Trigger "Prev" sketch when OSC argument contains `s="cp"`

[[sketch]]
file = "shader_stars.js" # Path relative to the TOML file
midi_note = 48          # Direct jump to this slot via MIDI C2
osc_key = "s"
osc_value = "hc"        # Direct jump to this slot when OSC arg contains `s="hc"`
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
| **Next Sketch** | `→` (Right Arrow) or MIDI/OSC navigation trigger |
| **Previous Sketch** | `←` (Left Arrow) or MIDI/OSC navigation trigger |
| **Direct Slot Jump** | Specific MIDI note or OSC trigger (`osc_key` & `osc_value`) |

---

## 🎨 Sketch API Reference

### Lifecycle Functions
Export these functions to define your sketch behavior:

| Function | When called | Argument | Return Value |
|---|---|---|---|
| `setup(scene)` | Once when the file is loaded | `scene` (Three.js `Scene`) | `config` object (Optional) |
| `update(context)` | Every frame (~60fps) | `context` — Data object | `void` |
| `cleanup(scene)` | Just before the sketch is replaced | `scene` (Three.js `Scene`) | `void` (Use `Shekere.clearScene(scene)` to reset) |

### Sketch Configuration
The `setup()` function can return an optional configuration object:

```js
export function setup(scene) {
  return {
    audio: {
      minFreqHz: 80,   // Lowest frequency to analyze
      maxFreqHz: 2000  // Highest frequency to analyze
    },
    renderer: {
      // Configure specific Tone Mapping per sketch. Default: THREE.NoToneMapping
      toneMapping: THREE.NeutralToneMapping, 
      toneMappingExposure: 1.0
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

## 🛠️ The `Shekere` Global Object

In addition to `THREE` and the `context` passed to `update()`, you can access the `Shekere` object for utilities:

| Member | Type | Description |
|---|---|---|
| `Shekere.clearScene(container)` | Function | Safely disposes all objects, geometries, and materials in a THREE.Object3D (usually the scene) to prevent memory leaks. |
| `Shekere.convertFileSrc(path)` | Function | Converts a local absolute file path to a URL that can be loaded by Three.js (e.g., for `TextureLoader`). |
| `Shekere.SKETCH_DIR` | String | The absolute path to the directory containing the currently active sketch. Useful for resolving relative paths for assets. |

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

## 🔊 Advanced Audio Features (Meyda)

Shekere includes integrated [Meyda.js](https://meyda.js.org/) for high-level audio feature extraction. These features allow for more semantic audio-reactivity (e.g., detecting "brightness" or "percussiveness").

#### Data Properties (`context.audio.features`)

| Property | Type | Description |
|---|---|---|
| `rms` | `number` | Root Mean Square. Accurate representation of perceived loudness. |
| `zcr` | `number` | Zero-Crossing Rate. High values indicate noisy/percussive sounds (hi-hats, etc). |
| `energy` | `number` | The total acoustic energy of the signal. |
| `spectralCentroid` | `number` | The "center of mass" of the spectrum. High values mean the sound is "brighter". |
| `spectralFlatness` | `number` | Indicates if a sound is tone-like (0.0) or noise-like (1.0). |
| `chroma` | `number[12]` | Intensity of the 12 pitch classes (C, C#, D, etc). |
| `mfcc` | `number[13]` | Mel-Frequency Cepstral Coefficients. Represents timbre/spectral shape. |

#### Example Usage

```js
export function update({ audio }) {
  const { rms, zcr, spectralCentroid } = audio.features;

  // Use ZCR to trigger a flash on percussive sounds
  if (zcr > 50) {
    this.flash = 1.0;
  }
}
```

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
- **Performance**: Always implement `cleanup()` to avoid memory leaks. The easiest way is to call `Shekere.clearScene(scene);`.
- **Dynamic Cleanup (Afterimages)**: If you want to create an "afterimage" effect where the old sketch persists, simply skip calling `Shekere.clearScene(scene)` or wrap it in an `if` statement (e.g., reactive to a MIDI fader).
- **Auto-Sync**: If you change an effect in code (e.g., `bloom.strength = 1.5`), the Control Panel slider will automatically move to match!
- **Hot Reload**: Keep your editor and the Visualizer side-by-side for the best experience.
- **Global Helper**: `Shekere.clearScene(scene)` and `THREE` are available globally. No imports needed.
