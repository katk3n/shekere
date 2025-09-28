<script>
  import { onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { preview, previewActions } from '../stores/preview.js';
  import { files } from '../stores/files.js';
  import ShaderCanvas from './ShaderCanvas.svelte';

  let previewContainer;
  let error = null;
  let previewData = null;
  let filesData = null;
  let shaderCanvas; // Reference to ShaderCanvas component
  let isWasmRendering = false; // WASM rendering state
  let loading = false;
  let wasmInitialized = false; // WASM initialization status

  // Reactive statements for store subscriptions
  $: previewData = $preview;
  $: filesData = $files;

  // Poll WASM initialization status
  function checkWasmStatus() {
    if (shaderCanvas) {
      const status = shaderCanvas.getInitializationStatus();
      const newWasmInitialized = status.isInitialized && status.wasmCore;

      if (newWasmInitialized !== wasmInitialized) {
        console.log('🔄 WASM status changed:', {
          old: wasmInitialized,
          new: newWasmInitialized,
          status
        });
        wasmInitialized = newWasmInitialized;
      }
    } else {
      console.log('⚠️ ShaderCanvas not available for status check');
    }
  }

  // Check WASM status periodically
  let statusInterval = setInterval(checkWasmStatus, 500);

  // Clean up interval on destroy
  onDestroy(() => {
    if (statusInterval) {
      clearInterval(statusInterval);
    }
  });

  async function startWasmPreview() {
    if (!previewData.config) {
      error = 'No configuration loaded. Please select a TOML file from the file tree.';
      return;
    }

    loading = true;
    error = null;

    try {
      console.log('🚀 Starting WASM preview with config:', previewData.config);

      // Check ShaderCanvas availability and status
      if (!shaderCanvas) {
        throw new Error('ShaderCanvas component not available');
      }

      const status = shaderCanvas.getInitializationStatus();
      console.log('📊 ShaderCanvas status:', status);

      if (!status.isInitialized || !status.wasmCore) {
        throw new Error('WASM core not properly initialized. Please wait a moment and try again.');
      }

      console.log('📡 Sending config and shader content to WASM canvas...');
      await shaderCanvas.loadConfiguration(previewData.config, previewData.shaderContent);
      isWasmRendering = true;
      console.log('✅ WASM preview started successfully');
    } catch (err) {
      error = `Failed to start WASM preview: ${err.message || err}`;
      console.error('❌ WASM preview error:', err);
    } finally {
      loading = false;
    }
  }

  function stopWasmPreview() {
    try {
      if (shaderCanvas) {
        shaderCanvas.stop();
        isWasmRendering = false;
        console.log('⏹️ WASM preview stopped');
      }
    } catch (err) {
      error = `Failed to stop WASM preview: ${err.message || err}`;
      console.error('❌ Stop WASM preview error:', err);
    }
  }

  $: canStart = previewData?.config && !isWasmRendering && !loading && wasmInitialized;
  $: canStop = isWasmRendering && !loading;
</script>

<div class="preview-window">
  <div class="preview-header">
    <div class="preview-title">
      <h3>WASM Shader Preview</h3>
      {#if previewData?.config}
        <div class="preview-status">
          <span class="status-badge"
                class:ready={!isWasmRendering && wasmInitialized}
                class:running={isWasmRendering}
                class:initializing={!wasmInitialized}>
            {isWasmRendering ? 'RUNNING' : wasmInitialized ? 'READY' : 'INITIALIZING'}
          </span>
          <span class="config-name">{filesData?.selectedFile || 'Unknown'}</span>
        </div>
      {/if}
    </div>
    <div class="preview-controls">
      <button
        on:click={startWasmPreview}
        disabled={!canStart}
        class="start-btn"
        class:disabled={!canStart}
      >
        {loading ? 'Starting...' : wasmInitialized ? 'Start Preview' : 'Initializing...'}
      </button>
      <button
        on:click={stopWasmPreview}
        disabled={!canStop}
        class="stop-btn"
        class:disabled={!canStop}
      >
        Stop
      </button>
    </div>
  </div>

  <!-- Preview Stats Bar -->
  {#if previewData?.config}
    <div class="preview-stats">
      <div class="stat-item">
        <span class="stat-label">Mode:</span>
        <span class="stat-value">WASM</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">Config:</span>
        <span class="stat-value">{filesData?.selectedFile || 'N/A'}</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">Resolution:</span>
        <span class="stat-value">{previewData.config.window?.width || 800}×{previewData.config.window?.height || 600}</span>
      </div>
    </div>
  {/if}

  <div class="preview-content" bind:this={previewContainer}>
    {#if error}
      <div class="error">
        <div class="error-title">❌ WASM Preview Error</div>
        <div class="error-message">{error}</div>
        <div class="error-actions">
          <button on:click={() => error = null}>Dismiss</button>
        </div>
      </div>
    {:else if previewData?.config}
      <div class="wasm-preview-active">
        <div class="wasm-canvas-container">
          <ShaderCanvas
            bind:this={shaderCanvas}
            width={previewData.config.window?.width || 800}
            height={previewData.config.window?.height || 600}
          />
        </div>
        <div class="wasm-info-panel">
          <h5>🌐 WASM Shader Renderer</h5>
          <div class="wasm-config-info">
            <div class="info-item">
              <span>Configuration:</span>
              <span>{filesData?.selectedFile || 'Unknown'}</span>
            </div>
            <div class="info-item">
              <span>Resolution:</span>
              <span>{previewData.config.window?.width || 800}×{previewData.config.window?.height || 600}</span>
            </div>
            <div class="info-item">
              <span>Pipelines:</span>
              <span>{previewData.config.pipeline?.length || 0}</span>
            </div>
            <div class="info-item">
              <span>Status:</span>
              <span class="status-indicator" class:active={isWasmRendering}>
                {isWasmRendering ? 'Rendering' : 'Stopped'}
              </span>
            </div>
          </div>
        </div>
      </div>
    {:else if previewData?.config}
      <div class="preview-ready">
        <div class="ready-content">
          <h4>✅ Configuration Loaded</h4>
          <p>Ready to start WASM preview with:</p>
          <div class="config-summary">
            <div class="config-item">
              <strong>File:</strong> {filesData?.selectedFile}
            </div>
            <div class="config-item">
              <strong>Resolution:</strong> {previewData.config.window?.width || 800}×{previewData.config.window?.height || 600}
            </div>
            <div class="config-item">
              <strong>Pipelines:</strong> {previewData.config.pipeline?.length || 0}
            </div>
          </div>
          <button on:click={startWasmPreview} class="large-start-btn" disabled={loading}>
            {loading ? 'Starting Preview...' : '▶ Start WASM Preview'}
          </button>
        </div>
      </div>
    {:else}
      <div class="preview-empty">
        <div class="empty-content">
          <h4>🌐 WASM Shader Preview</h4>
          <p>Select a TOML configuration file to start WASM-based shader rendering.</p>
          <div class="instructions">
            <ol>
              <li>Browse the file tree on the left</li>
              <li>Click on a <strong>.toml</strong> configuration file</li>
              <li>Click the <strong>Start Preview</strong> button to begin rendering</li>
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
    color: #4a9eff;
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

  .status-badge.ready {
    background-color: #28a745;
    color: white;
  }

  .status-badge.running {
    background-color: #4a9eff;
    color: white;
  }

  .status-badge.initializing {
    background-color: #ffc107;
    color: #000;
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

  .config-name {
    font-size: 0.75rem;
    color: #aaa;
    font-style: italic;
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

  /* WASM Mode Styles */
  .wasm-preview-active {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 1rem;
    gap: 1rem;
  }

  .wasm-canvas-container {
    flex: 1;
    display: flex;
    justify-content: center;
    align-items: center;
    background: #000;
    border-radius: 8px;
    border: 2px solid #4a9eff;
    min-height: 400px;
  }

  .wasm-info-panel {
    background: #2d2d2d;
    padding: 1rem;
    border-radius: 6px;
    border: 1px solid #444;
  }

  .wasm-info-panel h5 {
    margin: 0 0 0.75rem 0;
    color: #4a9eff;
    font-size: 1rem;
    font-weight: 600;
  }

  .wasm-config-info {
    display: flex;
    flex-wrap: wrap;
    gap: 1rem;
  }

  .info-item {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 120px;
  }

  .info-item span:first-child {
    font-size: 0.7rem;
    color: #aaa;
    text-transform: uppercase;
    font-weight: 500;
  }

  .info-item span:last-child {
    font-size: 0.85rem;
    color: #fff;
    font-weight: 600;
    font-family: monospace;
  }

  .status-indicator {
    color: #dc3545;
  }

  .status-indicator.active {
    color: #28a745;
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
    color: #4a9eff;
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
    border-left: 4px solid #4a9eff;
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
</style>