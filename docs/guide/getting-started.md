# Getting Started

Welcome to Shekere! This guide will help you get up and running with your first audio-reactive visuals in minutes.

## Installation

### Download the Binary (Recommended)
You can download the latest version of Shekere for your operating system from the [GitHub Releases](https://github.com/katk3n/shekere/releases) page.
- **macOS**: Download the `.dmg` file, open it, and drag Shekere to your Applications folder.
- **First Launch on macOS**: Since the app is currently unsigned, macOS will block it by default. To open it:
  1. Open the app and click **OK** on the warning dialog.
  2. Go to **System Settings** > **Privacy & Security**.
  3. Scroll down to the **Security** section and click **"Open Anyway"** for Shekere.

### Build from Source
If you prefer to build from source, you will need **Node.js** (v20+) and **Rust** installed.
```bash
# Clone the repository
git clone https://github.com/katk3n/shekere.git
cd shekere

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

## First Launch

When you first launch Shekere, your operating system will ask for **Microphone Permissions**.
- **Important**: You must grant this permission for Shekere to capture audio and generate reactive visuals. Shekere does not record or transmit your audio; it only analyzes it locally for visualization.

### 🛡️ Persistent Permissions (macOS)
If you find that Shekere asks for Microphone or File permissions **every time** you launch it, this is because the binary is "unsigned." macOS resets permissions for unsigned apps upon every restart.

To make permissions permanent, you can "re-sign" the app locally:
1. Move **Shekere.app** to your `/Applications` folder.
2. Open **Terminal** and run:
   ```bash
   # 1. Clear the "Quarantine" flag
   xattr -cr /Applications/Shekere.app

   # 2. Re-sign the app locally
   codesign --force --deep --sign - /Applications/Shekere.app
   ```

::: danger Security Warning
Re-signing a binary bypasses macOS Gatekeeper's checks. Only perform this on versions you have downloaded from the official repository or built yourself.
:::

## Creating Your First Sketch

Shekere provides a Three.js-based API. Instead of setting up a renderer yourself, you define your visuals by exporting specific lifecycle functions that Shekere calls.

Save the following as `my_first_sketch.js`:

```javascript
export function setup(scene) {
  // Setup your 3D objects
  const geometry = new THREE.IcosahedronGeometry(1, 2);
  const material = new THREE.MeshNormalMaterial({ wireframe: true });
  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

export function update({ time, audio }) {
  // This runs every frame (~60fps)
  this.mesh.rotation.y = time * 0.5;
  
  // React to audio volume (bass)
  const s = 1 + audio.bass;
  this.mesh.scale.set(s, s, s);
}

export function cleanup(scene) {
  // Clear the scene to prevent memory leaks
  Shekere.clearScene(scene);
}
```

Now, launch Shekere and use the **Control Panel** to load your file:
1. Two windows will appear: **Control Panel** and **Visualizer**.
2. In the **Control Panel**, click the **"Open Sketch"** button.
3. Select your `my_first_sketch.js` file.
4. You've just created your first 3D audio-reactive visual!

---

### Looking for more examples?
You can find more complex reference scripts covering MIDI, OSC, and Post-Processing in the [examples/](https://github.com/katk3n/shekere/tree/main/examples) directory of our GitHub repository.

Next: [Writing Sketches](./writing-sketches.md)
