<script>
  import { createEventDispatcher } from 'svelte';

  export let node;
  export let level = 0;
  export let expandedDirs;
  export let selectedPath;
  export let getFileIcon;

  const dispatch = createEventDispatcher();

  $: isExpanded = expandedDirs.has(node.path);
  $: isSelected = selectedPath === node.path;
  $: indentation = level * 1.5; // rem

  function handleClick() {
    if (node.is_directory) {
      dispatch('toggle', { dirPath: node.path });
    } else {
      dispatch('select', {
        detail: [node.path, node.name, node.file_type]
      });
    }
  }

  function handleToggle(event) {
    dispatch('toggle', event.detail);
  }

  function handleSelect(event) {
    dispatch('select', event.detail);
  }
</script>

<div class="tree-node" style="margin-left: {indentation}rem">
  <div
    class="tree-node-content"
    class:selected={isSelected}
    class:directory={node.is_directory}
    class:file={!node.is_directory}
    on:click={handleClick}
    on:keydown={(e) => e.key === 'Enter' && handleClick()}
    role="button"
    tabindex="0"
  >
    <!-- Directory toggle icon -->
    {#if node.is_directory}
      <span class="toggle-icon">
        {isExpanded ? '📂' : '📁'}
      </span>
    {:else}
      <span class="file-icon">
        {getFileIcon(node.file_type, node.is_directory)}
      </span>
    {/if}

    <!-- File/directory name -->
    <span class="node-name" class:toml-file={node.file_type === 'config'}>
      {node.name}
    </span>

    <!-- File type indicator for important files -->
    {#if node.file_type === 'config'}
      <span class="file-type-badge">TOML</span>
    {:else if node.file_type === 'shader'}
      <span class="file-type-badge">Shader</span>
    {/if}
  </div>

  <!-- Render children if directory is expanded -->
  {#if node.is_directory && isExpanded && node.children}
    {#each node.children as child}
      <svelte:self
        node={child}
        level={level + 1}
        {expandedDirs}
        {selectedPath}
        {getFileIcon}
        on:toggle={handleToggle}
        on:select={handleSelect}
      />
    {/each}
  {/if}
</div>

<style>
  .tree-node {
    user-select: none;
  }

  .tree-node-content {
    display: flex;
    align-items: center;
    padding: 0.25rem 0.5rem;
    cursor: pointer;
    border-radius: 3px;
    transition: background-color 0.15s;
    font-size: 0.85rem;
  }

  .tree-node-content:hover {
    background-color: #333;
  }

  .tree-node-content.selected {
    background-color: #007acc;
    color: white;
  }

  .tree-node-content.selected:hover {
    background-color: #005a9e;
  }

  .toggle-icon,
  .file-icon {
    margin-right: 0.5rem;
    font-size: 0.9rem;
    width: 1rem;
    text-align: center;
  }

  .node-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .node-name.toml-file {
    font-weight: 600;
    color: #4fc3f7;
  }

  .tree-node-content.selected .node-name.toml-file {
    color: white;
  }

  .file-type-badge {
    margin-left: 0.5rem;
    padding: 0.1rem 0.3rem;
    background-color: #444;
    color: #aaa;
    border-radius: 2px;
    font-size: 0.7rem;
    font-weight: 500;
    text-transform: uppercase;
  }

  .tree-node-content.selected .file-type-badge {
    background-color: rgba(255, 255, 255, 0.2);
    color: white;
  }

  .tree-node-content.directory {
    font-weight: 500;
  }

  .tree-node-content:focus {
    outline: 2px solid #007acc;
    outline-offset: 1px;
  }
</style>