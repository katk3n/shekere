# 2. Control Panel Activity Visualization

Date: 2026-04-09

## Status

Implemented (v0.5.1)

## Context

As Shekere grows as a live-coding audio-visual host, users frequently connect complex external input devices (Microphones, Audio Interfaces, MIDI Controllers, OSC from TidalCycles). Currently, the only way to verify that these signals are successfully arriving and being processed by the application is to write a test sketch and look at the Visualizer window.

This creates friction during live performance setups. The Control Panel UI should serve as a true "dashboard," providing immediate, clear feedback that signals are actively flowing into the system.

We define the requirements for two main features:
1. **Audio Mini-Visualizer**: A lightweight, real-time indicator of audio levels (Bass, Mid, High, Volume).
2. **MIDI / OSC Activity Indicators**: Lightweight UI indicators that flash upon receiving incoming events and display the last received message parameters (e.g., Note, CC, Address).

### Technical Challenge
According to the constraints set in [ADR 0001: Initial Architecture and Tech Stack](./0001-initial-architecture-and-tech-stack.md), and subsequent performance optimizations, **Audio processing (Web Audio API) happens exclusively in the Visualizer window**. Capturing audio twice (once in the Control Panel and once in the Visualizer) is unacceptable due to microphone exclusive-access limitations and performance penalties.

## Decision

We will implement "Signal Activity Monitors" in the Control Panel using the following architectural approach:

### 1. Audio Data Synchronization via IPC
Instead of running a separate audio context, the Control Panel will rely on the Visualizer window for audio data.
- The `visualizer.ts` process will extract lightweight summarization data (`volume`, `bass`, `mid`, `high`) from its local Web Audio pipeline.
- This data will be attached to an existing heavily-throttled IPC emission (`syncToHost`) or sent via a dedicated low-frequency IPC event (e.g., max 10-15 FPS) to avoid blocking the UI thread.
- `App.tsx` will receive this payload and render a set of horizontal "Level Meters" (using HTML/CSS `<progress>` or styled `<div>` elements) to represent the current frequency activity.

### 2. MIDI and OSC Global Subscription
Unlike Audio, MIDI and OSC data are captured directly by the Rust backend and emitted globally via Tauri's IPC (`app_handle.emit`).
- `App.tsx` will establish its own `listen` subscriptions to `midi-event` and `osc-event`, exactly as `visualizer.ts` does.
- Upon receiving an event, a "LED" indicator component will trigger a CSS animation (flash), and a small text field will show the most recent payload (e.g., `Note 36 Vel 127` or `/dirt/play`).
- To prevent UI lag from high-density MIDI/OSC streams, the displayed text state updates may be naturally chunked by React's rendering cycle or explicitly throttled if necessary, though LED flashes can be offloaded to CSS animations.

### 3. UI Placement
These monitors will be placed inside the Control Panel, grouped together in a "Signal Monitors" or "Activity" section, providing at-a-glance confidence before live performances.

## Consequences

### Positive
- **Improved UX/DX**: Users have immediate hardware troubleshooting feedback inside the control panel.
- **Architectural Integrity**: Maintains the ADR-0001 constraint where audio heavy-lifting is contained in the Visualizer.
- **Performance Preservation**: By throttling IPC synchronization to UI-friendly rates (10-15 FPS), we avoid dragging down React's render cycles.

### Negative
- **Minor IPC Overhead**: Passing audio arrays back over Tauri IPC at 10fps adds a trivial amount of overhead compared to not passing it at all. However, it is an acceptable trade-off for the UX gain.
