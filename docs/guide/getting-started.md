# Getting Started

Welcome to Shekere! This guide will help you get up and running with your first audio-reactive visuals in minutes.

## Installation

### Download the Binary (Recommended)
You can download the latest version of Shekere for your operating system from the [GitHub Releases](https://github.com/katk3n/shekere/releases) page.
- **macOS**: Download the `.dmg` file, open it, and drag Shekere to your Applications folder.

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

## Loading an Example Sketch

Shekere comes with several example sketches to help you get started.
1. Launch Shekere.
2. In the **Control Panel** window, click the **"Open Sketch"** button.
3. Navigate to the `examples/` directory in the Shekere source folder.
4. Select `hello_world.js` or `spectrum.js`.
5. You should see the visuals appear in the **Visualizer** window, reacting to any sound picked up by your microphone.

## Creating Your First Sketch

Creating a sketch for Shekere is as simple as writing a single JavaScript function. Save the following code as `my_first_sketch.js`:

```javascript
// Shekere looks for an 'update' function
export function update(ctx, width, height, audio) {
  // Clear the background
  ctx.fillStyle = 'black';
  ctx.fillRect(0, 0, width, height);

  // Use audio data (audio.rms is the volume)
  const radius = audio.rms * 500;

  // Draw a circle that reacts to sound
  ctx.beginPath();
  ctx.arc(width / 2, height / 2, radius, 0, Math.PI * 2);
  ctx.fillStyle = 'cyan';
  ctx.fill();
}
```

Now, go to the Control Panel, click **"Open Sketch"**, and select your new file. You've just created your first audio-reactive visual!

---

Next: [Writing Sketches](./writing-sketches.md) (Coming Soon)
