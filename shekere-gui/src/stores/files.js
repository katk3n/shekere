import { writable } from 'svelte/store';

// File tree state management
export const files = writable({
  root: null,
  selectedFile: null,
  selectedPath: null,
  loading: false,
  error: null
});

// Actions for file store
export const fileActions = {
  setFileTree: (fileTree) => {
    files.update(state => ({
      ...state,
      ...fileTree,  // Spread the FileTree directly instead of nesting it under 'root'
      loading: false,
      error: null
      // Selection state (selectedFile, selectedPath) is preserved
    }));
  },

  selectFile: (filePath, fileName) => {
    files.update(state => ({
      ...state,
      selectedFile: fileName,
      selectedPath: filePath
    }));
  },

  setLoading: (loading) => {
    files.update(state => ({
      ...state,
      loading
    }));
  },

  setError: (error) => {
    files.update(state => ({
      ...state,
      error,
      loading: false
    }));
  },

  clearSelection: () => {
    files.update(state => ({
      ...state,
      selectedFile: null,
      selectedPath: null
    }));
  }
};