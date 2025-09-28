# WASM IPC Integration Guide

This document explains how to integrate the Phase 2 WASM renderer with Tauri's IPC system for dynamic shader loading and hot reload functionality.

## Architecture Overview

```
┌─────────────────────┐       ┌─────────────────────┐       ┌─────────────────────┐
│   Tauri Backend     │       │      Tauri IPC     │       │  Browser Frontend   │
│                     │       │                     │       │                     │
│ ┌─────────────────┐ │  JSON │ ┌─────────────────┐ │  JSON │ ┌─────────────────┐ │
│ │Config/Shader    │ │ ───── │ │Message Routing  │ │ ───── │ │WASM Module      │ │
│ │Management       │ │       │ │& Serialization  │ │       │ │(shekere-core)   │ │
│ └─────────────────┘ │       │ └─────────────────┘ │       │ └─────────────────┘ │
│ ┌─────────────────┐ │       │                     │       │ ┌─────────────────┐ │
│ │Audio/MIDI       │ │ ───── │                     │ ───── │ │Enhanced WASM    │ │
│ │Processing       │ │       │                     │       │ │Renderer         │ │
│ └─────────────────┘ │       │                     │       │ └─────────────────┘ │
│ ┌─────────────────┐ │       │                     │       │                     │
│ │Hot Reload       │ │ ───── │                     │       │                     │
│ │System           │ │       │                     │       │                     │
│ └─────────────────┘ │       │                     │       │                     │
└─────────────────────┘       └─────────────────────┘       └─────────────────────┘
```

## JavaScript Integration Example

### 1. Initialize WASM Module

```javascript
import init, { WasmShekereCore } from './pkg/shekere_core.js';

let wasmCore = null;

async function initializeWASM() {
    // Initialize WASM module
    await init();

    // Create core instance
    wasmCore = new WasmShekereCore();

    // Get canvas element
    const canvas = document.getElementById('shader-canvas');

    // Initialize with canvas
    await wasmCore.init_with_canvas(canvas);

    console.log('✅ WASM initialized successfully');
}
```

### 2. Handle Tauri IPC Messages

```javascript
import { listen } from '@tauri-apps/api/event';

// Listen for configuration updates
await listen('config-update', (event) => {
    console.log('📦 Received config update:', event.payload);

    if (wasmCore) {
        const configJson = JSON.stringify(event.payload);
        wasmCore.initialize_with_config(configJson);
    }
});

// Listen for uniform data updates (real-time)
await listen('uniform-update', (event) => {
    if (wasmCore) {
        const message = {
            type: "UniformUpdate",
            data: event.payload
        };
        const messageJson = JSON.stringify(message);
        wasmCore.handle_ipc_message(messageJson);
    }
});

// Listen for hot reload events
await listen('hot-reload', (event) => {
    console.log('🔥 Hot reload event:', event.payload);

    if (wasmCore) {
        const message = {
            type: "HotReload",
            data: event.payload
        };
        const messageJson = JSON.stringify(message);
        wasmCore.handle_ipc_message(messageJson);
    }
});
```

### 3. Render Loop

```javascript
let animationFrameId = null;

function startRenderLoop() {
    function render() {
        if (wasmCore && wasmCore.is_initialized()) {
            try {
                wasmCore.render();
            } catch (error) {
                console.error('❌ Render error:', error);
            }
        }

        animationFrameId = requestAnimationFrame(render);
    }

    render();
}

function stopRenderLoop() {
    if (animationFrameId) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
    }
}
```

### 4. Handle User Input

```javascript
// Mouse input handling
canvas.addEventListener('mousemove', (event) => {
    if (wasmCore) {
        const rect = canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;

        wasmCore.handle_mouse_input(x, y);
    }
});

// Window resize handling
window.addEventListener('resize', () => {
    if (wasmCore && canvas) {
        const rect = canvas.getBoundingClientRect();
        canvas.width = rect.width;
        canvas.height = rect.height;

        wasmCore.resize(canvas.width, canvas.height);
    }
});
```

### 5. Monitor Statistics

```javascript
function showStats() {
    if (wasmCore) {
        const statsJson = wasmCore.get_stats();
        const stats = JSON.parse(statsJson);

        console.log('📊 WASM Stats:', {
            coreInitialized: stats.core_initialized,
            rendererInitialized: stats.renderer_initialized,
            shaderConfigs: stats.shader_configs,
            shaderCache: stats.shader_cache
        });
    }
}

// Show stats every 5 seconds
setInterval(showStats, 5000);
```

## IPC Message Types

### ConfigUpdate Message

```json
{
    "type": "ConfigUpdate",
    "data": {
        "shaders": [
            {
                "shader_type": "Fragment",
                "label": "Main Scene",
                "entry_point": "fs_main",
                "file": "scene.wgsl",
                "source": "@fragment\nfn fs_main() -> @location(0) vec4<f32> { ... }",
                "ping_pong": false,
                "persistent": false
            }
        ]
    }
}
```

### UniformUpdate Message

```json
{
    "type": "UniformUpdate",
    "data": {
        "time": 1.234,
        "delta_time": 0.016,
        "frame": 123,
        "resolution": [800, 600],
        "mouse": {
            "position": [0.5, 0.3],
            "buttons": [false, false, false],
            "wheel": 0.0
        },
        "spectrum": {
            "frequencies": [0.1, 0.2, ...],
            "amplitude": 0.8,
            "peak_frequency": 440.0,
            "sample_rate": 44100
        }
    }
}
```

### HotReload Message

```json
{
    "type": "HotReload",
    "data": {
        "file_path": "scene.wgsl",
        "new_content": "@fragment\nfn fs_main() -> @location(0) vec4<f32> { ... }"
    }
}
```

## Complete Integration Example

```javascript
// main.js
import init, { WasmShekereCore } from './pkg/shekere_core.js';
import { listen } from '@tauri-apps/api/event';

class ShekereWASMRenderer {
    constructor(canvasId) {
        this.canvasId = canvasId;
        this.wasmCore = null;
        this.animationFrameId = null;
    }

    async initialize() {
        // Initialize WASM
        await init();
        this.wasmCore = new WasmShekereCore();

        // Get canvas
        const canvas = document.getElementById(this.canvasId);
        await this.wasmCore.init_with_canvas(canvas);

        // Set up IPC listeners
        await this.setupIPC();

        // Set up input handlers
        this.setupInputHandlers(canvas);

        // Start render loop
        this.startRenderLoop();

        console.log('🎨 ShekereWASMRenderer initialized');
    }

    async setupIPC() {
        // Config updates
        await listen('config-update', (event) => {
            const configJson = JSON.stringify(event.payload);
            this.wasmCore.initialize_with_config(configJson);
        });

        // Uniform updates
        await listen('uniform-update', (event) => {
            const message = { type: "UniformUpdate", data: event.payload };
            this.wasmCore.handle_ipc_message(JSON.stringify(message));
        });

        // Hot reload
        await listen('hot-reload', (event) => {
            const message = { type: "HotReload", data: event.payload };
            this.wasmCore.handle_ipc_message(JSON.stringify(message));
        });
    }

    setupInputHandlers(canvas) {
        canvas.addEventListener('mousemove', (event) => {
            const rect = canvas.getBoundingClientRect();
            const x = event.clientX - rect.left;
            const y = event.clientY - rect.top;
            this.wasmCore.handle_mouse_input(x, y);
        });

        window.addEventListener('resize', () => {
            const rect = canvas.getBoundingClientRect();
            canvas.width = rect.width;
            canvas.height = rect.height;
            this.wasmCore.resize(canvas.width, canvas.height);
        });
    }

    startRenderLoop() {
        const render = () => {
            if (this.wasmCore && this.wasmCore.is_initialized()) {
                try {
                    this.wasmCore.render();
                } catch (error) {
                    console.error('Render error:', error);
                }
            }
            this.animationFrameId = requestAnimationFrame(render);
        };
        render();
    }

    destroy() {
        if (this.animationFrameId) {
            cancelAnimationFrame(this.animationFrameId);
        }
    }
}

// Usage
const renderer = new ShekereWASMRenderer('shader-canvas');
renderer.initialize();
```

## Next Steps

1. **Backend Integration**: Implement Tauri commands to send shader configurations and uniform data
2. **Performance Optimization**: Minimize IPC message frequency for real-time data
3. **Error Handling**: Add comprehensive error reporting between frontend and backend
4. **Testing**: Verify with existing shader examples from `examples/` directory
5. **Hot Reload**: Test shader modification and live updates

## Notes

- The WASM module uses console.log for debugging - check browser dev tools
- WebGPU requires browser flags to be enabled in development
- IPC messages are JSON-serialized for compatibility
- Mouse coordinates are automatically normalized within the WASM renderer
- All shader preprocessing happens in the WASM module using embedded common.wgsl definitions