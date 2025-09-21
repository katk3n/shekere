<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { preview } from '../stores/preview.js';

  let previewContainer;
  let loading = false;
  let error = null;

  async function startPreview() {
    loading = true;
    error = null;

    try {
      // TODO: Get config from selected file in Phase 4
      const mockConfig = {
        window: { width: 800, height: 600 },
        pipeline: []
      };

      const result = await invoke('start_preview', { config: mockConfig });
      preview.update(state => ({ ...state, handle: result, isRunning: true }));
    } catch (err) {
      error = err;
      console.error('Failed to start preview:', err);
    } finally {
      loading = false;
    }
  }

  async function stopPreview() {
    try {
      await invoke('stop_preview');
      preview.update(state => ({ ...state, handle: null, isRunning: false }));
    } catch (err) {
      error = err;
      console.error('Failed to stop preview:', err);
    }
  }
</script>

<div class="preview-window">
  <div class="preview-header">
    <h3>Shader Preview</h3>
    <div class="preview-controls">
      <button on:click={startPreview} disabled={loading}>
        {loading ? 'Starting...' : 'Start Preview'}
      </button>
      <button on:click={stopPreview}>Stop</button>
    </div>
  </div>

  <div class="preview-content" bind:this={previewContainer}>
    {#if error}
      <div class="error">
        Preview error: {error}
        <button on:click={() => { error = null; }}>Dismiss</button>
      </div>
    {:else}
      <div class="placeholder">
        <div class="placeholder-content">
          <h4>WebGPU Rendering Preview</h4>
          <p>
            WebGPU integration will be implemented in Phase 4.
            <br />
            This area will display real-time shader rendering.
          </p>
          <div class="mock-canvas">
            <div class="mock-shader-output">
              <!-- Placeholder for shader rendering -->
              <div class="gradient-demo"></div>
            </div>
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
    background: white;
  }

  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    border-bottom: 1px solid #eee;
    background-color: #fafafa;
  }

  .preview-header h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: #555;
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
  }

  .preview-controls button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .preview-controls button:hover:not(:disabled) {
    background: #005a9e;
  }

  .preview-content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .error {
    padding: 1rem;
    background-color: #fee;
    color: #d73a49;
    border: 1px solid #fcc;
  }

  .error button {
    margin-left: 0.5rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid #d73a49;
    border-radius: 3px;
    background: white;
    color: #d73a49;
    cursor: pointer;
    font-size: 0.75rem;
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