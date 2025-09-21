<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { files } from '../stores/files.js';

  let loading = false;
  let error = null;

  onMount(async () => {
    await loadFileTree('.');
  });

  async function loadFileTree(path) {
    loading = true;
    error = null;

    try {
      const result = await invoke('get_directory_tree', { path });
      files.set(result);
    } catch (err) {
      error = err;
      console.error('Failed to load file tree:', err);
    } finally {
      loading = false;
    }
  }

  function handleFileSelect(filePath) {
    // TODO: Implement file selection logic in Phase 4
    console.log('File selected:', filePath);
  }
</script>

<div class="file-tree">
  <div class="file-tree-header">
    <h3>Project Files</h3>
    <button on:click={() => loadFileTree('.')} disabled={loading}>
      Refresh
    </button>
  </div>

  <div class="file-tree-content">
    {#if loading}
      <div class="loading">Loading files...</div>
    {:else if error}
      <div class="error">
        Error loading files: {error}
        <button on:click={() => loadFileTree('.')}>Retry</button>
      </div>
    {:else}
      <div class="placeholder">
        File tree implementation will be completed in Phase 4.
        <br />
        Current structure shows basic component setup.
      </div>
    {/if}
  </div>
</div>

<style>
  .file-tree {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .file-tree-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    border-bottom: 1px solid #eee;
  }

  .file-tree-header h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: #555;
  }

  .file-tree-header button {
    padding: 0.25rem 0.5rem;
    border: 1px solid #ccc;
    border-radius: 3px;
    background: white;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .file-tree-header button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .file-tree-content {
    flex: 1;
    padding: 0.75rem;
    overflow-y: auto;
  }

  .loading {
    color: #666;
    font-style: italic;
  }

  .error {
    color: #d73a49;
    font-size: 0.8rem;
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
    padding: 1rem;
    background-color: #f8f9fa;
    border: 1px dashed #ccc;
    border-radius: 4px;
    color: #666;
    font-size: 0.8rem;
    text-align: center;
    line-height: 1.4;
  }
</style>