# Architecture Decision Record (ADR): Shekere

Building the next-generation audiovisual environment for live coding.

## 1. Status

Implemented (v0.4.0)

## 2. Context & Goals

The legacy "Shekere (v1)" was built using Rust and WGSL, but faced challenges with high learning curves for users and high development/maintenance costs for the host application.
The "Shekere (v2)" project aims to achieve the following:
- Reference (Legacy Shekere (v1) repository): https://github.com/katk3n/shekere-legacy

1. **Broaden Target Audience**: Pivot from low-level shader languages (WGSL) to web standards (JavaScript / Three.js) to minimize the learning curve.
2. **Accelerate & Stabilize Development**: Adopt a modern hybrid architecture that avoids complex system programming and leverages Web Standard APIs and Tauri v2 plugins.

## 3. Core Paradigm

Shekere is not just an "app that displays fixed visuals"; it is a **"host environment (runner) that dynamically loads JavaScript code written by users in external editors and renders it with Three.js in sync with music."**

## 4. System Architecture

- **Desktop Framework**: Tauri v2
- **Build Tool**: Vite
- **Language**:
    - TypeScript (UI and Front-end logic)
    - Rust (Implementation of features that cannot be achieved with TypeScript)

The system must strictly adhere to the following division of roles (boundaries).

### 4.1 Front-end (TypeScript / WebView)

Centralize all processing here except for OSC reception. Keep Rust code to a minimum.

- **UI Framework**: Modern Vite ecosystem libraries (React, Vue, or Vanilla TS).
- **Rendering Engine**: Three.js (Standard API + Post-processing with EffectComposer).
- **Audio Analysis**: Uses Web Audio API (FFT analysis and low-frequency detection via AnalyserNode).
- **MIDI Input**: **Web MIDI API is not supported on macOS (WKWebView) and thus will not be used.** (See 4.2 for details)
- **File Watching**: Uses the official Tauri v2 plugin `@tauri-apps/plugin-fs` to monitor (`watch`) and read (`readTextFile`) user file changes directly from the TypeScript side.

### 4.2 Back-end (Rust / Tauri Core)

Responsible only for tasks that cannot be executed directly from the front-end due to web constraints.

- **OSC Communication**: Uses the `rosc` crate to receive OSC messages via UDP sockets. Emits received data to the front-end via Tauri `emit`.
- **MIDI Input**: Uses the `midir` crate. To bypass WKWebView limitations, the Rust side monitors all ports and sends data to the front-end via `emit`.
- **Note**: Do not perform file watching (e.g., `notify`) in Rust.

### 5. Multi-Window Design

For live performance use cases, two windows must be launched and managed separately.

1. **Control Panel (Main Window)**:
    - Renders UI and selects files (determines watch paths).
    - Maintains the Web Audio API instance and performs data analysis.
    - Sends acquired/analyzed data to the Visualizer every frame via Tauri IPC (`emit`).
    - (MIDI and OSC are sent directly to the Visualizer from the Rust side)
2. **Visualizer (Sub-window for Rendering)**:
    - A window without UI, intended for full-screen output.
    - Maintains the Three.js Canvas.
    - Receives data from the Control Panel and OSC data (listen), injects it into user code, and updates rendering.

### 6. Dynamic Module Loading Strategy

To bypass Vite's bundle constraints and safely hot-reload raw user JS files locally, the **Blob URL pattern** is adopted.

```typescript
// Implementation hint: Hot-reloading using Blob URL
const codeString = await readTextFile(userFilePath); // Read via Tauri fs plugin
const blob = new Blob([codeString], { type: 'application/javascript' });
const blobUrl = URL.createObjectURL(blob);

if (currentModule) currentModule.cleanup(); // Disposal of existing Three.js objects, etc.

// Dynamically import user module and pass Three.js scene
const userModule = await import(/* @vite-ignore */ blobUrl);
userModule.setup(scene);
```

### 7. Interface Contract

Interface specifications between the host app (Shekere) and user code. The AI agent must implement a loader that conforms to this.

```typescript
// JS file written by the user (Example: my_sketch.js) 
export function setup(scene) {
    this.mesh = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshNormalMaterial());
    scene.add(this.mesh);
}

export function update(context) {
    const { time, audio, midi, osc } = context;
    // Example: Scale change based on bass (audio.bass)
    const scale = 1.0 + (audio.bass * 2.0);
    this.mesh.scale.set(scale, scale, scale);
}

// Mandatory: Cleanup function to prevent memory leaks during hot-reloading
export function cleanup(scene) {
    scene.remove(this.mesh);
    this.mesh.geometry.dispose();
    this.mesh.material.dispose();
}
```

### 8. Implementation Roadmap for AI Agent

The AI agent must proceed with implementation and verification step-by-step according to the following phases.

#### Phase 1: Bootstrapping [Completed in v0.1.0]

- Establishment of Tauri v2 + Vite + TS foundation using `create-tauri-app`, etc.
- Implementation of settings (`tauri.conf.json`, etc.) to launch two windows: Control Panel and Visualizer.

#### Phase 2: File Watching & Dynamic Loading [Completed in v0.2.0]

- Implementation of the JS file monitoring mechanism using `@tauri-apps/plugin-fs`.
- Establishment of Three.js foundation on the Visualizer side and implementation of dynamic JS file execution via Blob URL.

#### Phase 3: Data Pipeline (Audio, MIDI & OSC) [Completed in v0.3.0]

- Microphone permission handling and Web Audio API (FFT) implementation in the Control Panel.
- Implementation of MIDI input (`midir`) support.
- Implementation of OSC reception via UDP sockets in Rust and `emit` to the front-end.
- Integration of the pipeline to inject various data into `update(context)`.

#### Phase 4: Post-Processing & UI [Completed in v0.4.0]

- Introduction of EffectComposer (`UnrealBloomPass`, etc.) to the Visualizer.
- Implementation of parameter adjustment UI (Bloom intensity, etc.) in the Control Panel and synchronization between windows.

### 9. Constraints (Strict Adherence Mandatory)

- **Rust Audio/MIDI Crate Usage Restrictions**: `cpal`, `rustfft`, etc., are prohibited for their original purposes. However, implementing MIDI input via `midir` on the Rust side is permitted as an exception to bypass the lack of Web MIDI support in macOS browsers.
- **No Rust File Watching Crates**: Do not use file watching crates such as `notify`. Call `@tauri-apps/plugin-fs` from the front-end.
- **No custom compilers**: Do not attempt to build or implement custom parsing or compilation for WGSL or GLSL. User code is treated as pure JavaScript.