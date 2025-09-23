<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { preview, previewActions } from '../stores/preview.js';
  import { files } from '../stores/files.js';

  let previewContainer;
  let loading = false;
  let error = null;
  let previewData = null;
  let filesData = null;
  let statusCheckInterval = null;

  // Canvas-related variables
  let renderCanvas;
  let canvasWidth = 800;
  let canvasHeight = 600;
  let canvasReady = false;
  let webgpuSupported = false;

  // Subscribe to store changes
  preview.subscribe(value => {
    previewData = value;
  });

  files.subscribe(value => {
    filesData = value;
  });

  onMount(() => {
    // Check preview status when component mounts
    checkPreviewStatus();

    // Set up periodic status checking
    statusCheckInterval = setInterval(checkPreviewStatus, 2000);
  });

  onDestroy(() => {
    if (statusCheckInterval) {
      clearInterval(statusCheckInterval);
    }
    stopFrameUpdateLoop();
  });

  async function checkPreviewStatus() {
    try {
      const status = await invoke('get_preview_status');
      if (status) {
        previewActions.setHandle(status);
        previewActions.updateStats(status.fps, status.render_time_ms);
      } else {
        previewActions.stop();
      }
    } catch (err) {
      // Silently handle - preview might not be running
    }
  }

  async function startPreview() {
    if (!previewData.config) {
      error = 'No configuration loaded. Please select a TOML file from the file tree.';
      return;
    }

    loading = true;
    error = null;
    previewActions.setError(null);

    try {
      const result = await invoke('start_preview', {
        config: previewData.config,
        configPath: filesData?.selectedPath || null
      });

      previewActions.setHandle(result);
      console.log('Preview started:', result);
    } catch (err) {
      error = err;
      previewActions.setError(err);
      console.error('Failed to start preview:', err);
    } finally {
      loading = false;
    }
  }

  async function stopPreview() {
    try {
      await invoke('stop_preview');
      previewActions.stop();
      stopFrameUpdateLoop();
    } catch (err) {
      error = err;
      previewActions.setError(err);
      console.error('Failed to stop preview:', err);
    }
  }

  // Canvas initialization and WebGPU setup
  async function initializeCanvas() {
    if (!renderCanvas) return;

    try {
      // Check WebGPU support (but continue with canvas setup regardless)
      console.log('navigator.gpu available:', !!navigator.gpu);
      if (!navigator.gpu) {
        console.warn('WebGPU not supported, but continuing with canvas setup');
        webgpuSupported = false;
        // Don't return - continue with canvas setup for fallback rendering
      } else {
        webgpuSupported = true;
      }
      console.log('WebGPU supported, getting canvas dimensions...');

      // Get canvas dimensions from backend or use defaults
      try {
        const dimensions = await invoke('get_canvas_dimensions');
        console.log('Canvas dimensions from backend:', dimensions);
        if (dimensions) {
          canvasWidth = dimensions[0];
          canvasHeight = dimensions[1];
        } else {
          // Fallback dimensions if no preview is running yet
          canvasWidth = 800;
          canvasHeight = 600;
        }
      } catch (err) {
        console.warn('Failed to get canvas dimensions from backend, using defaults:', err);
        canvasWidth = 800;
        canvasHeight = 600;
      }

      renderCanvas.width = canvasWidth;
      renderCanvas.height = canvasHeight;

      // Basic canvas setup
      const context = renderCanvas.getContext('2d');
      console.log('Canvas 2D context:', !!context);
      if (context) {
        // Clear canvas with a dark background
        context.fillStyle = '#1a1a1a';
        context.fillRect(0, 0, canvasWidth, canvasHeight);

        // Add a simple visualization indicator
        context.fillStyle = '#007acc';
        context.font = '16px Arial';
        context.textAlign = 'center';
        context.fillText('WebGPU Renderer Active', canvasWidth / 2, canvasHeight / 2);
        context.fillText(`${canvasWidth}×${canvasHeight}`, canvasWidth / 2, canvasHeight / 2 + 25);
      }

      canvasReady = true;
    } catch (err) {
      console.error('Failed to initialize canvas:', err);
      webgpuSupported = false;
    }
  }

  let frameUpdateInterval = null;

  async function startFrameUpdateLoop() {
    if (frameUpdateInterval) return; // Already running

    frameUpdateInterval = setInterval(async () => {
      if (!previewData?.isRunning || !renderCanvas) {
        stopFrameUpdateLoop();
        return;
      }

      try {
        const frameDataWithDims = await invoke('get_frame_data_with_dimensions');
        if (frameDataWithDims && frameDataWithDims.data && frameDataWithDims.data.length > 0) {
          // Update canvas dimensions if they've changed
          if (frameDataWithDims.width !== canvasWidth || frameDataWithDims.height !== canvasHeight) {
            canvasWidth = frameDataWithDims.width;
            canvasHeight = frameDataWithDims.height;
            renderCanvas.width = canvasWidth;
            renderCanvas.height = canvasHeight;
          }
          displayFrameData(frameDataWithDims.data);
        }
      } catch (err) {
        console.warn('Failed to get frame data with dimensions:', err);
        // Fallback to old method
        try {
          const frameData = await invoke('get_frame_data');
          if (frameData && frameData.length > 0) {
            displayFrameData(frameData);
          }
        } catch (fallbackErr) {
          console.warn('Failed to get frame data (fallback):', fallbackErr);
        }
      }
    }, 33); // ~30 FPS
  }

  function stopFrameUpdateLoop() {
    if (frameUpdateInterval) {
      clearInterval(frameUpdateInterval);
      frameUpdateInterval = null;
    }
  }

  function displayFrameData(data) {
    if (!renderCanvas) return;

    const context = renderCanvas.getContext('2d');
    if (!context) return;

    // Validate data size before creating ImageData
    const expectedSize = canvasWidth * canvasHeight * 4; // RGBA = 4 bytes per pixel
    if (data.length !== expectedSize) {
      console.warn(`Frame data size mismatch: expected ${expectedSize} bytes but got ${data.length} bytes`);
      console.warn(`Canvas dimensions: ${canvasWidth}x${canvasHeight}`);

      // Try to handle the mismatch gracefully
      if (data.length === 0) {
        // Clear canvas if no data
        context.fillStyle = '#1a1a1a';
        context.fillRect(0, 0, canvasWidth, canvasHeight);
        return;
      }

      // If data size doesn't match, don't try to create ImageData
      return;
    }

    try {
      // Create ImageData from the frame data
      const imageData = new ImageData(
        new Uint8ClampedArray(data),
        canvasWidth,
        canvasHeight
      );

      // Draw the image data to canvas
      context.putImageData(imageData, 0, 0);
    } catch (error) {
      console.error('Failed to create or display ImageData:', error);
      console.error('Data length:', data.length, 'Expected:', expectedSize);
      console.error('Canvas dimensions:', canvasWidth, 'x', canvasHeight);
    }
  }

  // Mouse event handlers
  async function handleMouseMove(event) {
    if (!previewData?.isRunning) return;

    const rect = renderCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    try {
      await invoke('handle_mouse_input', { x, y });
    } catch (err) {
      console.warn('Failed to send mouse input:', err);
    }
  }

  async function handleMouseEnter(event) {
    if (!previewData?.isRunning) return;

    const rect = renderCanvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    try {
      await invoke('handle_mouse_input', { x, y });
    } catch (err) {
      console.warn('Failed to send mouse enter:', err);
    }
  }

  async function handleMouseLeave() {
    if (!previewData?.isRunning) return;

    try {
      // Send mouse coordinates outside canvas bounds to indicate mouse left
      await invoke('handle_mouse_input', { x: -1, y: -1 });
    } catch (err) {
      console.warn('Failed to send mouse leave:', err);
    }
  }

  // Reactive statements
  $: {
    if (previewData?.isRunning && renderCanvas && !canvasReady) {
      initializeCanvas();
    }
  }

  // Separate reactive statement for frame update loop
  $: {
    if (previewData?.isRunning && renderCanvas && canvasReady) {
      startFrameUpdateLoop();
    } else if (!previewData?.isRunning) {
      stopFrameUpdateLoop();
    }
  }

  $: canStartPreview = previewData?.config && !previewData?.isRunning && !loading;
  $: canStopPreview = previewData?.isRunning;
</script>

<div class="preview-window">
  <div class="preview-header">
    <div class="preview-title">
      <h3>Shader Preview</h3>
      {#if previewData?.handle}
        <div class="preview-status">
          <span class="status-badge running">RUNNING</span>
          <span class="config-name">{filesData?.selectedFile || 'Unknown'}</span>
        </div>
      {/if}
    </div>
    <div class="preview-controls">
      <button
        on:click={startPreview}
        disabled={!canStartPreview || loading}
        class="start-btn"
        class:disabled={!canStartPreview || loading}
      >
        {loading ? 'Starting...' : 'Start Preview'}
      </button>
      <button
        on:click={stopPreview}
        disabled={!canStopPreview}
        class="stop-btn"
        class:disabled={!canStopPreview}
      >
        Stop
      </button>
    </div>
  </div>

  <!-- Preview Stats Bar -->
  {#if previewData?.handle}
    <div class="preview-stats">
      <div class="stat-item">
        <span class="stat-label">FPS:</span>
        <span class="stat-value">{previewData.fps.toFixed(1)}</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">Render Time:</span>
        <span class="stat-value">{previewData.renderTime.toFixed(1)}ms</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">ID:</span>
        <span class="stat-value">{previewData.handle.id}</span>
      </div>
    </div>
  {/if}

  <div class="preview-content" bind:this={previewContainer}>
    {#if error || previewData?.error}
      <div class="error">
        <div class="error-title">❌ Preview Error</div>
        <div class="error-message">{error || previewData?.error}</div>
        <div class="error-actions">
          <button on:click={() => { error = null; previewActions.setError(null); }}>
            Dismiss
          </button>
          {#if previewData?.config}
            <button on:click={startPreview} class="retry-btn">
              Retry
            </button>
          {/if}
        </div>
      </div>
    {:else if previewData?.isRunning}
      <div class="preview-active">
        <div class="preview-canvas-placeholder">
          <div class="canvas-info">
            <h4>🎨 Shader Rendering Active</h4>
            <p>Configuration: <strong>{filesData?.selectedFile || 'Unknown'}</strong></p>
            <p>Resolution: <strong>{previewData.config?.window?.width}×{previewData.config?.window?.height}</strong></p>
            <p>Pipelines: <strong>{previewData.config?.pipeline?.length || 0}</strong></p>

            <div class="pipeline-info">
              {#if previewData.config?.pipeline}
                <h5>Active Pipelines:</h5>
                <ul>
                  {#each previewData.config.pipeline as pipeline, index}
                    <li>
                      <span class="pipeline-name">{pipeline.label || `Pipeline ${index + 1}`}</span>
                      <span class="pipeline-details">({pipeline.shader_type} - {pipeline.file})</span>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>

            <!-- Real WebGPU Canvas -->
            <div class="canvas-container">
              <canvas
                bind:this={renderCanvas}
                class="webgpu-canvas"
                width={canvasWidth}
                height={canvasHeight}
                on:mousemove={handleMouseMove}
                on:mouseenter={handleMouseEnter}
                on:mouseleave={handleMouseLeave}
              />
              <div class="canvas-overlay" class:hidden={canvasReady}>
                <span>Initializing WebGPU Renderer...</span>
                <span class="note">Real-time shader preview</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    {:else if previewData?.config}
      <div class="preview-ready">
        <div class="ready-content">
          <h4>✅ Configuration Loaded</h4>
          <p>Ready to start preview with:</p>
          <div class="config-summary">
            <div class="config-item">
              <strong>File:</strong> {filesData?.selectedFile}
            </div>
            <div class="config-item">
              <strong>Resolution:</strong> {previewData.config.window.width}×{previewData.config.window.height}
            </div>
            <div class="config-item">
              <strong>Pipelines:</strong> {previewData.config.pipeline.length}
            </div>
          </div>
          <button on:click={startPreview} class="large-start-btn" disabled={loading}>
            {loading ? 'Starting Preview...' : '▶ Start Preview'}
          </button>
        </div>
      </div>
    {:else}
      <div class="preview-empty">
        <div class="empty-content">
          <h4>📁 No Configuration Loaded</h4>
          <p>Select a TOML configuration file from the file tree to start previewing shaders.</p>
          <div class="instructions">
            <ol>
              <li>Browse the file tree on the left</li>
              <li>Click on a <strong>.toml</strong> configuration file</li>
              <li>Click "Start Preview" to begin rendering</li>
            </ol>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .preview-window {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #1e1e1e;
    color: #ffffff;
  }

  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    border-bottom: 1px solid #444;
    background-color: #252525;
  }

  .preview-title {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .preview-title h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: #ffffff;
  }

  .preview-status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .status-badge {
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    font-size: 0.7rem;
    font-weight: 600;
    text-transform: uppercase;
  }

  .status-badge.running {
    background-color: #28a745;
    color: white;
  }

  .config-name {
    font-size: 0.75rem;
    color: #aaa;
    font-style: italic;
  }

  .preview-controls {
    display: flex;
    gap: 0.5rem;
  }

  .preview-controls button {
    padding: 0.25rem 0.75rem;
    border: 1px solid #007acc;
    border-radius: 3px;
    background: #007acc;
    color: white;
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.2s;
  }

  .preview-controls button.start-btn {
    background: #28a745;
    border-color: #28a745;
  }

  .preview-controls button.start-btn:hover:not(.disabled) {
    background: #218838;
  }

  .preview-controls button.stop-btn {
    background: #dc3545;
    border-color: #dc3545;
  }

  .preview-controls button.stop-btn:hover:not(.disabled) {
    background: #c82333;
  }

  .preview-controls button.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preview-stats {
    display: flex;
    justify-content: space-around;
    padding: 0.5rem;
    background-color: #252525;
    border-bottom: 1px solid #444;
    font-size: 0.8rem;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.1rem;
  }

  .stat-label {
    color: #aaa;
    font-size: 0.7rem;
    text-transform: uppercase;
    font-weight: 500;
  }

  .stat-value {
    color: #ffffff;
    font-weight: 600;
    font-family: monospace;
  }

  .preview-content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  /* Error State */
  .error {
    padding: 1.5rem;
    background-color: #3d1a1a;
    color: #ff6b6b;
    border: 1px solid #5a1f1f;
    text-align: center;
  }

  .error-title {
    font-size: 1.1rem;
    font-weight: 600;
    margin-bottom: 0.5rem;
  }

  .error-message {
    margin-bottom: 1rem;
    font-size: 0.9rem;
    line-height: 1.4;
  }

  .error-actions {
    display: flex;
    justify-content: center;
    gap: 0.5rem;
  }

  .error-actions button {
    padding: 0.4rem 0.8rem;
    border: 1px solid #ff6b6b;
    border-radius: 3px;
    background: #2d2d2d;
    color: #ff6b6b;
    cursor: pointer;
    font-size: 0.75rem;
    transition: all 0.2s;
  }

  .error-actions button:hover {
    background: #ff6b6b;
    color: white;
  }

  .retry-btn {
    background: #ff6b6b !important;
    color: white !important;
  }

  .retry-btn:hover {
    background: #fa5252 !important;
  }

  /* Preview Active State */
  .preview-active {
    padding: 1rem;
    height: 100%;
    overflow-y: auto;
  }

  .canvas-info h4 {
    margin: 0 0 1rem 0;
    color: #28a745;
    font-size: 1.1rem;
  }

  .canvas-info p {
    margin: 0.5rem 0;
    font-size: 0.9rem;
  }

  .pipeline-info {
    margin: 1rem 0;
    padding: 0.75rem;
    background-color: #2d2d2d;
    border-radius: 4px;
  }

  .pipeline-info h5 {
    margin: 0 0 0.5rem 0;
    font-size: 0.85rem;
    color: #cccccc;
  }

  .pipeline-info ul {
    margin: 0;
    padding-left: 1rem;
  }

  .pipeline-info li {
    margin-bottom: 0.25rem;
    font-size: 0.8rem;
  }

  .pipeline-name {
    font-weight: 600;
    color: #ffffff;
  }

  .pipeline-details {
    color: #aaaaaa;
    font-style: italic;
  }

  .canvas-placeholder {
    margin-top: 1rem;
  }

  .animated-border {
    position: relative;
    border: 2px solid #007acc;
    border-radius: 8px;
    overflow: hidden;
    height: 200px;
  }

  /* New WebGPU Canvas Styles */
  .canvas-container {
    position: relative;
    margin-top: 1rem;
    border: 2px solid #007acc;
    border-radius: 8px;
    overflow: hidden;
    background: #1a1a1a;
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 300px;
  }

  .webgpu-canvas {
    display: block;
    max-width: 100%;
    max-height: 100%;
    border-radius: 6px;
    background: #1a1a1a;
  }

  .canvas-overlay {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    text-align: center;
    background: rgba(45, 45, 45, 0.95);
    color: #ffffff;
    padding: 1rem;
    border-radius: 4px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    z-index: 10;
    transition: opacity 0.3s ease;
  }

  .canvas-overlay.hidden {
    opacity: 0;
    pointer-events: none;
  }

  .canvas-overlay .note {
    display: block;
    font-size: 0.75rem;
    color: #aaaaaa;
    margin-top: 0.5rem;
  }

  /* Preview Ready State */
  .preview-ready {
    padding: 2rem;
    text-align: center;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .ready-content h4 {
    margin: 0 0 1rem 0;
    color: #28a745;
    font-size: 1.2rem;
  }

  .config-summary {
    margin: 1rem 0;
    padding: 1rem;
    background-color: #2d2d2d;
    border-radius: 4px;
    text-align: left;
  }

  .config-item {
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }

  .large-start-btn {
    padding: 0.75rem 1.5rem;
    font-size: 1rem;
    background: #28a745;
    border: 1px solid #28a745;
    color: white;
    border-radius: 4px;
    cursor: pointer;
    margin-top: 1rem;
    transition: all 0.2s;
  }

  .large-start-btn:hover:not(:disabled) {
    background: #218838;
  }

  .large-start-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Preview Empty State */
  .preview-empty {
    padding: 2rem;
    text-align: center;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-content h4 {
    margin: 0 0 1rem 0;
    color: #cccccc;
    font-size: 1.2rem;
  }

  .empty-content p {
    margin-bottom: 1.5rem;
    color: #aaaaaa;
    line-height: 1.4;
  }

  .instructions {
    text-align: left;
    background-color: #2d2d2d;
    padding: 1rem;
    border-radius: 4px;
    border-left: 4px solid #007acc;
  }

  .instructions ol {
    margin: 0;
    padding-left: 1.2rem;
  }

  .instructions li {
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
    line-height: 1.4;
  }

  .placeholder {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
  }

  .placeholder-content {
    text-align: center;
    color: #666;
  }

  .placeholder-content h4 {
    margin: 0 0 0.5rem 0;
    color: #333;
  }

  .placeholder-content p {
    margin: 0 0 1rem 0;
    line-height: 1.4;
    font-size: 0.9rem;
  }

  .mock-canvas {
    width: 400px;
    height: 300px;
    border: 2px solid #007acc;
    border-radius: 8px;
    overflow: hidden;
    margin: 0 auto;
  }

  .mock-shader-output {
    width: 100%;
    height: 100%;
    position: relative;
  }

  .gradient-demo {
    width: 100%;
    height: 100%;
    background: linear-gradient(45deg,
      #ff006e, #ff7700, #ffcc00, #8338ec, #3a86ff);
    background-size: 200% 200%;
    animation: gradientShift 3s ease-in-out infinite;
  }

  @keyframes gradientShift {
    0% { background-position: 0% 50%; }
    50% { background-position: 100% 50%; }
    100% { background-position: 0% 50%; }
  }
</style>