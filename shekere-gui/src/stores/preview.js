import { writable } from 'svelte/store';

// Preview state management
export const preview = writable({
  handle: null,
  isRunning: false,
  config: null,
  error: null,
  fps: 0,
  renderTime: 0
});

// Actions for preview store
export const previewActions = {
  setHandle: (handle) => {
    preview.update(state => ({
      ...state,
      handle,
      isRunning: !!handle
    }));
  },

  setConfig: (config) => {
    preview.update(state => ({
      ...state,
      config
    }));
  },

  setRunning: (isRunning) => {
    preview.update(state => ({
      ...state,
      isRunning
    }));
  },

  setError: (error) => {
    preview.update(state => ({
      ...state,
      error
    }));
  },

  updateStats: (fps, renderTime) => {
    preview.update(state => ({
      ...state,
      fps,
      renderTime
    }));
  },

  stop: () => {
    preview.update(state => ({
      ...state,
      handle: null,
      isRunning: false,
      config: null
    }));
  }
};