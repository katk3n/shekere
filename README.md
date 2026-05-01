# Shekere

<p align="center">
  <img src="src/assets/shekere-logo.png" width="400" alt="Shekere logo">
</p>

**Shekere** is a high-performance live-coding environment for creating interactive audio-visual art with JavaScript and Three.js.

Built with **Tauri**, **React**, and **Rust**, Shekere provides a seamless bridge between real-time sound analysis, MIDI, OSC, and 3D graphics. Write your sketches in any text editor and see the results instantly with hot-reloading.

---

## 📖 Documentation

For full installation guides, API references, and tutorials, please visit our documentation site:

👉 **[https://katk3n.github.io/shekere/](https://katk3n.github.io/shekere/)**

*(Japanese version available: [https://katk3n.github.io/shekere/ja/](https://katk3n.github.io/shekere/ja/))*

---

## 🚀 Quick Start

### 1. Download & Launch
Download the latest version for macOS from the [GitHub Releases](https://github.com/katk3n/shekere/releases) page.
- Open the `.dmg` file and drag Shekere to your Applications folder.
- **Note**: Since the app is currently unsigned, you may need to allow it in **System Settings > Privacy & Security**.

### 2. Build from Source
If you prefer to build from source, you will need **Node.js** (v20+) and **Rust**.

```bash
git clone https://github.com/katk3n/shekere.git
cd shekere
npm install
npm run tauri dev
```

---

## ✨ Key Features

- **High Performance**: Native performance powered by Rust and Tauri.
- **Web Tech Stack**: Use familiar JavaScript/TypeScript and Three.js.
- **Audio Reactive**: Built-in Meyda.js integration for deep spectral analysis.
- **Live Editing**: Hot-reload sketches instantly upon saving.
- **MIDI & OSC**: Seamless integration with external controllers and software (TidalCycles, etc.).
- **Post-Processing**: Professional-grade effects (Bloom, RGB Shift, etc.) with bidirectional sync between UI and Code.

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
