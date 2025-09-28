<script>
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  let canvas;
  let wasmCore = null;
  let animationFrameId = null;
  let isInitialized = false;
  let isRendering = false;
  let initializationPromise = null;
  let resizeObserver = null;

  // Canvas properties
  export let width = 800;
  export let height = 600;

  onMount(async () => {
    console.log('🔄 ShaderCanvas onMount - starting initialization...');
    initializationPromise = initializeWasm();

    // Wait for initialization to complete and log the result
    try {
      await initializationPromise;
      console.log('✅ ShaderCanvas initialization completed successfully');
    } catch (error) {
      console.error('❌ ShaderCanvas initialization failed:', error);
    }
  });

  async function initializeWasm() {
    try {
      console.log('🚀 Initializing WASM shader canvas...');

      // Import WASM module
      console.log('📦 Importing WASM module...');
      const wasmModule = await import('../../pkg/shekere-core.js');
      console.log('📦 WASM module imported successfully');

      console.log('🔧 Initializing WASM default...');
      await wasmModule.default(); // Initialize WASM
      console.log('🔧 WASM default initialized');

      // Create WASM core instance
      console.log('🏗️ Creating WasmShekereCore instance...');
      wasmCore = new wasmModule.WasmShekereCore();
      console.log('🏗️ WasmShekereCore created');

      // Wait for canvas to be available and properly attached to DOM
      if (!canvas) {
        console.log('⏳ Waiting for canvas element...');
        await new Promise(resolve => {
          const checkCanvas = () => {
            if (canvas) {
              resolve();
            } else {
              setTimeout(checkCanvas, 100);
            }
          };
          checkCanvas();
        });
      }

      // Additional check: ensure canvas is properly attached to DOM and sized
      console.log('🔍 Waiting for canvas to be fully ready...');
      await new Promise(resolve => {
        const checkCanvasReady = () => {
          if (canvas &&
              canvas.offsetParent !== null &&
              canvas.width > 0 &&
              canvas.height > 0) {
            console.log('✅ Canvas is fully ready');
            resolve();
          } else {
            console.log('⏳ Canvas not ready yet, waiting...', {
              canvas: !!canvas,
              offsetParent: canvas?.offsetParent,
              width: canvas?.width,
              height: canvas?.height
            });
            setTimeout(checkCanvasReady, 100);
          }
        };
        checkCanvasReady();
      });

      // Initialize with canvas
      console.log('🎨 Initializing WASM with canvas...');
      console.log('🎨 Canvas element:', canvas);
      console.log('🎨 Canvas dimensions:', canvas.width, 'x', canvas.height);

      // Initialize with canvas - the WASM code will handle WebGPU/WebGL2 fallback automatically
      console.log('🔍 Canvas readiness check...');
      console.log('🔍 Canvas parentNode:', canvas.parentNode);
      console.log('🔍 Canvas style.display:', canvas.style.display);

      console.log('🎨 Attempting WASM initialization with automatic WebGPU/WebGL2 fallback...');

      try {
        await wasmCore.init_with_canvas(canvas);
        console.log('🎨 Canvas initialization complete');

        // Immediately check if initialization was successful
        const isWasmInitialized = wasmCore.is_initialized();
        console.log('🔍 WASM is_initialized() after init_with_canvas:', isWasmInitialized);

        if (!isWasmInitialized) {
          console.error('❌ WASM core reports as not initialized after init_with_canvas!');
        }
      } catch (canvasError) {
        console.error('❌ Canvas initialization failed with error:', canvasError);
        throw canvasError;
      }

      // Set up IPC listeners
      console.log('📡 Setting up IPC listeners...');
      await setupIPC();
      console.log('📡 IPC listeners setup complete');

      // Set up input handlers
      console.log('🎮 Setting up input handlers...');
      setupInputHandlers();
      console.log('🎮 Input handlers setup complete');

      isInitialized = true;
      console.log('✅ WASM shader canvas fully initialized');
      return true;
    } catch (error) {
      console.error('❌ Failed to initialize WASM:', error);
      isInitialized = false;
      throw error;
    }
  }

  onDestroy(() => {
    stop();
    if (resizeObserver) {
      resizeObserver.disconnect();
      resizeObserver = null;
    }
  });

  // Load configuration and start rendering
  export async function loadConfiguration(config, shaderContent = null) {
    try {
      console.log('⏳ Waiting for WASM initialization...');

      // Wait for initialization to complete
      if (initializationPromise) {
        await initializationPromise;
      }

      if (!wasmCore || !isInitialized) {
        throw new Error('WASM core not initialized after waiting');
      }

      console.log('📡 Loading configuration into WASM:', { config, shaderContent });

      // Get shader source from loaded shader content or use default
      let shaderSource = "// Default shader\n@fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(0.5, 0.8, 1.0, 1.0); }";

      if (shaderContent && Object.keys(shaderContent).length > 0) {
        // Use the first available shader from the loaded content
        const firstShaderKey = Object.keys(shaderContent)[0];
        shaderSource = shaderContent[firstShaderKey];
        console.log('🎨 Using loaded shader:', firstShaderKey, shaderSource.length, 'characters');
      } else {
        console.log('⚠️ No shader content provided, using default shader');
      }

      // Convert config to IPC format and send to WASM core
      const configUpdate = {
        type: "ConfigUpdate",
        data: {
          shader_config: {
            shader_source: shaderSource,
            entry_point: config.pipeline?.[0]?.entry_point || "fs_main",
            shader_type: "Fragment",
            label: config.pipeline?.[0]?.label || "Main Shader"
          },
          hot_reload: null,
          error: null
        }
      };

      console.log('📤 Sending IPC message to WASM:', JSON.stringify(configUpdate, null, 2));
      await wasmCore.handle_ipc_message(JSON.stringify(configUpdate));

      // Start rendering loop
      startRenderLoop();

      console.log('✅ Configuration loaded and rendering started');
    } catch (error) {
      console.error('❌ Failed to load configuration:', error);
      throw error;
    }
  }

  // Stop rendering
  export function stop() {
    if (animationFrameId) {
      cancelAnimationFrame(animationFrameId);
      animationFrameId = null;
    }
    isRendering = false;
    console.log('⏹️ Rendering stopped');
  }

  async function setupIPC() {
    // Listen for configuration updates from Tauri backend
    await listen('config-update', (event) => {
      console.log('📦 Received config update:', event.payload);

      if (wasmCore) {
        try {
          const configJson = JSON.stringify(event.payload);
          wasmCore.initialize_with_config(configJson);
          isInitialized = true;
        } catch (error) {
          console.error('❌ Failed to handle config update:', error);
        }
      }
    });

    // Listen for uniform data updates (real-time audio/MIDI data)
    await listen('uniform-update', (event) => {
      if (wasmCore) {
        try {
          const message = {
            type: "UniformUpdate",
            data: event.payload
          };
          wasmCore.handle_ipc_message(JSON.stringify(message));
        } catch (error) {
          console.error('❌ Failed to handle uniform update:', error);
        }
      }
    });

    // Listen for hot reload events
    await listen('hot-reload', (event) => {
      console.log('🔥 Hot reload event:', event.payload);

      if (wasmCore) {
        try {
          const message = {
            type: "HotReload",
            data: event.payload
          };
          wasmCore.handle_ipc_message(JSON.stringify(message));
        } catch (error) {
          console.error('❌ Failed to handle hot reload:', error);
        }
      }
    });
  }

  function setupInputHandlers() {
    // Mouse move handler
    canvas.addEventListener('mousemove', (event) => {
      if (wasmCore) {
        const rect = canvas.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        wasmCore.handle_mouse_input(x, y);
      }
    });

    // Resize handler - create ResizeObserver if not already created
    if (!resizeObserver) {
      resizeObserver = new ResizeObserver(entries => {
        if (wasmCore && entries.length > 0) {
          const entry = entries[0];
          const { width: newWidth, height: newHeight } = entry.contentRect;

          canvas.width = newWidth;
          canvas.height = newHeight;

          wasmCore.resize(newWidth, newHeight);
        }
      });

      resizeObserver.observe(canvas);
    }
  }

  function startRenderLoop() {
    if (isRendering) {
      console.log('🔄 Render loop already running');
      return;
    }

    isRendering = true;
    console.log('🎬 Starting render loop');

    function render() {
      if (!isRendering) return;

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

  // Get statistics for debugging
  function getStats() {
    if (wasmCore) {
      try {
        const statsJson = wasmCore.get_stats();
        const stats = JSON.parse(statsJson);
        console.log('📊 WASM Stats:', stats);
        return stats;
      } catch (error) {
        console.error('❌ Failed to get stats:', error);
        return null;
      }
    }
    return null;
  }

  // Export for parent component access
  export { getStats };

  // Export initialization status for parent component
  export function getInitializationStatus() {
    // Check actual WASM core initialization status
    const wasmCoreInitialized = wasmCore ? wasmCore.is_initialized() : false;

    console.log('🔍 ShaderCanvas getInitializationStatus:', {
      wasmCore: !!wasmCore,
      wasmCoreInitialized,
      localInitialized: isInitialized,
      isRendering
    });

    return {
      isInitialized: wasmCoreInitialized, // Use actual WASM status
      wasmCore: !!wasmCore,
      isRendering,
      localInitialized: isInitialized // Keep local status for debugging
    };
  }
</script>

<canvas
  bind:this={canvas}
  {width}
  {height}
  class="shader-canvas"
  class:initialized={isInitialized}
  data-webgpu="true"
/>

<style>
  .shader-canvas {
    border: 1px solid #333;
    background: #000;
    cursor: crosshair;
    transition: border-color 0.3s ease;
  }

  .shader-canvas.initialized {
    border-color: #4a9eff;
  }

  .shader-canvas:hover {
    border-color: #6bb6ff;
  }
</style>