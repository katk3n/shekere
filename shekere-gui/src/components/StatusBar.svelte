<script>
  import { files } from '../stores/files.js';
  import { preview } from '../stores/preview.js';

  $: selectedFile = $files?.selectedFile || null;
  $: previewStatus = $preview?.isRunning ? 'Running' : 'Stopped';
  $: totalFiles = $files?.root?.total_files || 0;
</script>

<div class="status-bar">
  <div class="status-section">
    <span class="status-label">Selected:</span>
    <span class="status-value">
      {selectedFile || 'No file selected'}
    </span>
  </div>

  <div class="status-section">
    <span class="status-label">Files:</span>
    <span class="status-value">{totalFiles}</span>
  </div>

  <div class="status-section">
    <span class="status-label">Preview:</span>
    <span class="status-value preview-status" class:running={$preview?.isRunning}>
      {previewStatus}
    </span>
  </div>

  <div class="status-section">
    <span class="status-label">Phase:</span>
    <span class="status-value">3 - GUI Foundation</span>
  </div>
</div>

<style>
  .status-bar {
    display: flex;
    align-items: center;
    padding: 0.4rem 1rem;
    background-color: #f8f9fa;
    border-top: 1px solid #dee2e6;
    font-size: 0.75rem;
    min-height: 24px;
  }

  .status-section {
    display: flex;
    align-items: center;
    margin-right: 1rem;
  }

  .status-label {
    font-weight: 500;
    color: #6c757d;
    margin-right: 0.25rem;
  }

  .status-value {
    color: #495057;
  }

  .preview-status {
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    background-color: #dc3545;
    color: white;
    font-weight: 500;
  }

  .preview-status.running {
    background-color: #28a745;
  }
</style>