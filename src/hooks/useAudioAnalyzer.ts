import { useEffect, useRef, useState, useCallback } from 'react';
import { emit, listen } from '@tauri-apps/api/event';

const FFT_SIZE = 4096;       // frequencyBinCount = 2048, ~10.8 Hz/bin @ 44.1kHz
const BAND_COUNT = 256;      // ~385 bins within piano range -> 256 bands (~1.5 bins/band)

// Default frequency range (Piano A0 to C8)
const DEFAULT_MIN_FREQ = 27.5;
const DEFAULT_MAX_FREQ = 4186;

// Frequency boundaries for bass / mid / high (Hz)
const BASS_MAX_HZ = 250;    // 0 ~ 250 Hz : Kick, bass, etc.
const MID_MAX_HZ = 2_000;   // 250 ~ 2kHz : Mid-range

export interface AudioData {
  volume: number;
  bass: number;   // Average of defined min freq to 250 Hz
  mid: number;    // Average of 250 Hz to 2,000 Hz
  high: number;   // Average of 2,000 Hz to defined max freq
  bands: number[]; // 256 bands, 0.0~1.0 each
}

export function useAudioAnalyzer() {
  const [isActive, setIsActive] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // For dynamic configuration
  const minFreqRef = useRef(DEFAULT_MIN_FREQ);
  const maxFreqRef = useRef(DEFAULT_MAX_FREQ);

  const audioContextRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const sourceRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const requestAnimationFrameRef = useRef<number | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const dataArrayRef = useRef<any>(null);

  const cleanup = useCallback(() => {
    if (requestAnimationFrameRef.current) {
      cancelAnimationFrame(requestAnimationFrameRef.current);
      requestAnimationFrameRef.current = null;
    }
    if (sourceRef.current) {
      sourceRef.current.disconnect();
      sourceRef.current = null;
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach(track => track.stop());
      streamRef.current = null;
    }
    if (audioContextRef.current) {
      audioContextRef.current.close().catch(console.error);
      audioContextRef.current = null;
    }
    analyserRef.current = null;
    dataArrayRef.current = null;
    setIsActive(false);
  }, []);

  const analyzeAndEmit = useCallback(() => {
    const analyser = analyserRef.current;
    const dataArray = dataArrayRef.current;
    const sampleRate = audioContextRef.current?.sampleRate ?? 44100;
    if (!analyser || !dataArray) return;

    analyser.getByteFrequencyData(dataArray);

    // --- Get current configuration ---
    const minFreq = minFreqRef.current;
    const maxFreq = maxFreqRef.current;

    // --- Calculate valid bin range ---
    const binResolution = sampleRate / FFT_SIZE; // Hz/bin

    // --- Calculate 256 bands (logarithmic scale) ---
    const bands: number[] = new Array(BAND_COUNT);
    const logRatio = Math.pow(maxFreq / minFreq, 1 / BAND_COUNT);

    for (let b = 0; b < BAND_COUNT; b++) {
      const freqStart = minFreq * Math.pow(logRatio, b);
      const freqEnd = minFreq * Math.pow(logRatio, b + 1);

      const binStart = Math.floor(freqStart / binResolution);
      const binEnd = Math.max(binStart + 1, Math.floor(freqEnd / binResolution));

      let sum = 0;
      let count = 0;
      for (let i = binStart; i < binEnd; i++) {
        if (i < dataArray.length) {
          sum += dataArray[i];
          count++;
        }
      }
      
      // Base average value (0.0 - 1.0)
      let val = count > 0 ? (sum / count) / 255.0 : 0;

      // --- Adjust sensitivity and contrast ---
      // 1. High-frequency compensation (Tilt EQ): Boost gain as frequency increases (1.0 to 1.8x)
      const tilt = 1.0 + (b / BAND_COUNT) * 0.8;
      val *= tilt;

      // 2. Non-linear scaling (Power curve): Suppress noise and emphasize clear sounds
      bands[b] = Math.min(1.0, Math.pow(val, 1.5));
    }

    // --- Calculate bass / mid / high ---
    const getFrequencyIndex = (f: number) => {
        if (f <= minFreq) return 0;
        if (f >= maxFreq) return BAND_COUNT;
        return Math.floor(Math.log(f / minFreq) / Math.log(logRatio));
    };

    const bassEndBand = getFrequencyIndex(BASS_MAX_HZ);
    const midEndBand = getFrequencyIndex(MID_MAX_HZ);

    const sumRange = (arr: number[], from: number, to: number) => {
      let s = 0;
      const start = Math.max(0, from);
      const end = Math.min(arr.length, to);
      if (start >= end) return 0;
      for (let i = start; i < end; i++) s += arr[i];
      return s / (end - start);
    };

    const audioData: AudioData = {
      volume: bands.reduce((a, b) => a + b, 0) / BAND_COUNT,
      bass: sumRange(bands, 0, bassEndBand),
      mid: sumRange(bands, bassEndBand, midEndBand),
      high: sumRange(bands, midEndBand, BAND_COUNT),
      bands,
    };

    emit('audio-data', audioData).catch(console.error);

    requestAnimationFrameRef.current = requestAnimationFrame(analyzeAndEmit);
  }, []);

  const start = useCallback(async () => {
    setError(null);
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true, video: false });
      streamRef.current = stream;

      const AudioContextCtor = window.AudioContext || (window as any).webkitAudioContext;
      const ctx = new AudioContextCtor();
      audioContextRef.current = ctx;

      const analyser = ctx.createAnalyser();
      analyser.fftSize = FFT_SIZE;
      analyser.smoothingTimeConstant = 0.5;
      
      // Adjust sensitivity: Suppress noise and add contrast
      analyser.minDecibels = -70; // Default is -100 (raised to cut more noise)
      analyser.maxDecibels = -10; // Default is -30 (raised to widen dynamic range)
      analyserRef.current = analyser;
      dataArrayRef.current = new Uint8Array(analyser.frequencyBinCount);

      const source = ctx.createMediaStreamSource(stream);
      source.connect(analyser);
      sourceRef.current = source;

      setIsActive(true);
      analyzeAndEmit();
    } catch (err: any) {
      console.error('Failed to start audio analyzer:', err);
      setError(`Failed to initialize microphone: ${err.message || err}`);
      cleanup();
    }
  }, [analyzeAndEmit, cleanup]);

  const stop = useCallback(() => {
    cleanup();
  }, [cleanup]);

  useEffect(() => {
    const unlisten = listen<{ minFreqHz?: number; maxFreqHz?: number }>('audio-config-update', (event) => {
        if (event.payload.minFreqHz !== undefined) minFreqRef.current = event.payload.minFreqHz;
        if (event.payload.maxFreqHz !== undefined) maxFreqRef.current = event.payload.maxFreqHz;
        console.log(`Audio analysis range updated: ${minFreqRef.current}Hz - ${maxFreqRef.current}Hz`);
    });

    return () => {
      cleanup();
      unlisten.then(fn => fn());
    };
  }, [cleanup]);

  return { isActive, start, stop, error };
}
