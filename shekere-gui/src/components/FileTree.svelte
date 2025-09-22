<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { files, fileActions } from '../stores/files.js';
  import { preview, previewActions } from '../stores/preview.js';
  import FileTreeNode from './FileTreeNode.svelte';

  // Component state
  let loading = false;
  let error = null;
  let fileTreeData = null;
  let expandedDirs = new Set();
  let currentDirectory = '.';

  // Local selection state - managed independently from store
  let selectedFile = null;

  // Subscribe to file store changes (data only, not selection state)
  files.subscribe(value => {
    fileTreeData = value;
    console.log('File store updated:', value);
  });

  onMount(async () => {
    await loadFileTree('.');
  });

  async function loadFileTree(path) {
    loading = true;
    error = null;
    fileActions.setLoading(true);

    try {
      const result = await invoke('get_directory_tree', { path });
      console.log('File tree result:', result);

      // Set the FileTree result directly to the store
      fileActions.setFileTree(result);
      currentDirectory = path;

      // Expand root directory by default
      if (result && result.root) {
        expandedDirs.add(result.root.path);
        expandedDirs = expandedDirs;
      }
    } catch (err) {
      error = err;
      fileActions.setError(err);
      console.error('Failed to load file tree:', err);
    } finally {
      loading = false;
      fileActions.setLoading(false);
    }
  }


  async function selectDirectory() {
    try {
      // Open native directory selection dialog
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: currentDirectory,
        title: 'Select Root Directory'
      });

      if (selected && typeof selected === 'string') {
        console.log('Selected directory:', selected);

        // Clear selection when changing directory
        selectedFile = null;
        fileActions.clearSelection();

        await loadFileTree(selected);

        // Reset expanded directories for new directory
        expandedDirs = new Set();
      }
      // If selected is null, user cancelled - no action needed
    } catch (err) {
      error = err;
      console.error('Failed to select directory:', err);
    }
  }


  async function handleFileSelect(filePath, fileName, fileType) {
    // Update local selection state (managed independently from store)
    selectedFile = { path: filePath, name: fileName, type: fileType };

    console.log('File selected:', filePath);

    // Update store for other components
    fileActions.selectFile(filePath, fileName);

    // If it's a TOML file, load the configuration
    if (fileType === 'config') {
      try {
        const config = await invoke('load_toml_config', { path: filePath });
        previewActions.setConfig(config);
        console.log('Loaded TOML config:', config);
      } catch (err) {
        console.error('Failed to load TOML config:', err);
        previewActions.setError(`Failed to load ${fileName}: ${err}`);
      }
    }
  }

  function toggleDirectory(dirPath) {
    if (expandedDirs.has(dirPath)) {
      expandedDirs.delete(dirPath);
    } else {
      expandedDirs.add(dirPath);
    }
    expandedDirs = expandedDirs;
  }

  function getFileIcon(fileType, isDirectory) {
    if (isDirectory) return '📁';

    switch (fileType) {
      case 'config': return '⚙️';
      case 'shader': return '🎨';
      case 'rust': return '🦀';
      case 'javascript': return '📜';
      case 'typescript': return '📘';
      case 'json': return '📄';
      case 'markdown': return '📝';
      case 'text': return '📄';
      default: return '📄';
    }
  }

  function isFileSelected(filePath) {
    return selectedFile?.path === filePath;
  }
</script>

<div class="file-tree">
  <div class="file-tree-header">
    <div class="header-top">
      <h3>Project Files</h3>
      <div class="header-controls">
        <button
          on:click={selectDirectory}
          disabled={loading}
          class="select-dir-btn"
          type="button"
        >
          📁 Browse
        </button>
        <button on:click={() => loadFileTree(currentDirectory)} disabled={loading}>
          🔄 Refresh
        </button>
      </div>
    </div>
    <div class="current-path">
      <span class="path-label">Path:</span>
      <span class="path-value" title={currentDirectory}>{currentDirectory}</span>
    </div>

  </div>

  <div class="file-tree-content">
    {#if loading}
      <div class="loading">Loading files...</div>
    {:else if error}
      <div class="error">
        Error loading files: {error}
        <button on:click={() => loadFileTree('.')}>Retry</button>
      </div>
    {:else if fileTreeData?.root}
      <!-- Render the actual file tree -->
      {#if fileTreeData.root.root?.children}
        {#each fileTreeData.root.root.children as child}
          <FileTreeNode
            node={child}
            level={0}
            {expandedDirs}
            selectedPath={selectedFile?.path}
            on:select={(e) => handleFileSelect(...e.detail.detail)}
            on:toggle={(e) => toggleDirectory(e.detail.dirPath)}
            {getFileIcon}
          />
        {/each}
      {/if}

      <!-- File tree stats -->
      <div class="file-tree-stats">
        {fileTreeData?.root?.total_directories || 0} directories, {fileTreeData?.root?.total_files || 0} files
      </div>
    {:else}
      <div class="empty-tree">
        No files found in the current directory.
      </div>
    {/if}
  </div>

</div>

<style>
  .file-tree {
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: #1e1e1e;
    color: #ffffff;
  }

  .file-tree-header {
    padding: 0.75rem;
    border-bottom: 1px solid #444;
    background-color: #252525;
  }

  .header-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .header-controls {
    display: flex;
    gap: 0.5rem;
  }

  .file-tree-header h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: #ffffff;
  }

  .file-tree-header button {
    padding: 0.25rem 0.5rem;
    border: 1px solid #555;
    border-radius: 3px;
    background: #333;
    color: #ffffff;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .file-tree-header button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .file-tree-header button:hover:not(:disabled) {
    background: #444;
  }

  .select-dir-btn {
    background: #007acc !important;
    border-color: #007acc !important;
    pointer-events: auto !important;
    cursor: pointer !important;
    z-index: 1000 !important;
    position: relative !important;
    min-width: 80px !important;
    min-height: 30px !important;
  }

  .select-dir-btn:hover:not(:disabled) {
    background: #005a9e !important;
  }

  .select-dir-btn:disabled {
    background: #555 !important;
    border-color: #555 !important;
    cursor: not-allowed !important;
  }

  .current-path {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem;
    background-color: #1e1e1e;
    border-radius: 3px;
    border: 1px solid #444;
  }

  .path-label {
    font-size: 0.75rem;
    color: #aaaaaa;
    font-weight: 500;
  }

  .path-value {
    font-size: 0.75rem;
    color: #ffffff;
    font-family: monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }



  .file-tree-content {
    flex: 1;
    padding: 0.75rem;
    overflow-y: auto;
    background-color: #1e1e1e;
  }

  .loading {
    color: #aaa;
    font-style: italic;
  }

  .error {
    color: #ff6b6b;
    font-size: 0.8rem;
  }

  .error button {
    margin-left: 0.5rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid #ff6b6b;
    border-radius: 3px;
    background: #333;
    color: #ff6b6b;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .error button:hover {
    background: #ff6b6b;
    color: #ffffff;
  }


  .file-tree-stats {
    margin-top: 1rem;
    padding: 0.5rem;
    font-size: 0.75rem;
    color: #aaa;
    text-align: center;
    border-top: 1px solid #444;
    background-color: #252525;
  }

  .empty-tree {
    padding: 2rem 1rem;
    text-align: center;
    color: #aaa;
    font-size: 0.9rem;
    font-style: italic;
  }
</style>