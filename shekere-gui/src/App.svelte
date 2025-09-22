<script>
  import FileTree from './components/FileTree.svelte'
  import PreviewWindow from './components/PreviewWindow.svelte'
  import StatusBar from './components/StatusBar.svelte'

  import { files } from './stores/files.js'
  import { preview } from './stores/preview.js'
</script>

<main class="app">
  <div class="app-header">
    <h1>shekere</h1>
    <div class="menu">
      <!-- Menu items will be added in later phases -->
    </div>
  </div>

  <div class="app-body">
    <div class="sidebar">
      <FileTree />
    </div>

    <div class="main-content">
      <PreviewWindow />
    </div>
  </div>

  <StatusBar />
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background-color: #1e1e1e;
    color: #ffffff;
  }

  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 1rem;
    background-color: #2d2d2d;
    border-bottom: 1px solid #444;
  }

  .app-header h1 {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 600;
    color: #ffffff;
  }

  .app-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: 250px;
    min-width: 200px;
    max-width: 400px;
    background-color: #1e1e1e;
    border-right: 1px solid #444;
    resize: horizontal;
    overflow: auto;
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background-color: #1e1e1e;
  }
</style>