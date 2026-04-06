import { useState, useCallback, useEffect } from 'react';
import { emit } from '@tauri-apps/api/event';

/**
 * Audio analyzer hook for the Control Panel.
 *
 * All audio capture and FFT analysis now runs in the Visualizer window
 * to avoid costly per-frame IPC. This hook only manages UI state and
 * sends lightweight start/stop commands to the Visualizer.
 */
export function useAudioAnalyzer() {
  const [isActive, setIsActive] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const start = useCallback(async () => {
    setError(null);
    try {
      await emit('start-audio');
      setIsActive(true);
    } catch (err: any) {
      console.error('Failed to emit start-audio:', err);
      setError(`Failed to start audio: ${err.message ?? err}`);
    }
  }, []);

  const stop = useCallback(async () => {
    try {
      await emit('stop-audio');
    } catch (err: any) {
      console.error('Failed to emit stop-audio:', err);
    }
    setIsActive(false);
  }, []);

  // Ensure audio is stopped when the Control Panel unmounts
  useEffect(() => {
    return () => {
      if (isActive) {
        emit('stop-audio').catch(console.error);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isActive]);

  return { isActive, start, stop, error };
}
