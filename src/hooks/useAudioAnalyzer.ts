import { useEffect, useRef, useState, useCallback } from 'react';
import { emit } from '@tauri-apps/api/event';

const FFT_SIZE = 4096;       // frequencyBinCount = 2048, ~10.8 Hz/bin @ 44.1kHz
const BAND_COUNT = 256;      // ピアノ音域内で ~385ビン→256バンド (≈1.5ビン/バンド)
// ピアノの音域（A0〜C8）で実用的な周波数範囲 (Hz)
const MIN_FREQ_HZ = 27.5;   // A0 (ピアノ最低音)
const MAX_FREQ_HZ = 4_186;  // C8 (ピアノ最高音)
// bass / mid / high の帯域境界
const BASS_MAX_HZ = 250;    // 0 ~ 250 Hz  : キック、ベース等（ピアノ低音域）
const MID_MAX_HZ = 2_000;   // 250 ~ 2kHz  : 中音域（C4〜C7付近）
// high: 2kHz ~ 4186Hz    : 高音域（C7以上〜C8）

export interface AudioData {
  volume: number;
  bass: number;   // 27.5〜250 Hz の平均（ピアノ低音域）
  mid: number;    // 250〜2,000 Hz の平均（ピアノ中音域）
  high: number;   // 2,000〜4,186 Hz の平均（ピアノ高音域）
  bands: number[]; // 256バンド（27.5Hz〜4186Hz）、各 0.0~1.0
}

export function useAudioAnalyzer() {
  const [isActive, setIsActive] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

    // --- 有効なビン範囲を計算（実用周波数範囲のみ使用）---
    // 各 FFT ビンが対応する周波数幅: sampleRate / fftSize
    const binResolution = sampleRate / FFT_SIZE; // Hz/bin
    const minBin = Math.max(0, Math.round(MIN_FREQ_HZ / binResolution));
    const maxBin = Math.min(
      Math.floor(MAX_FREQ_HZ / binResolution),
      (FFT_SIZE / 2) - 1
    );
    const usefulBinCount = maxBin - minBin + 1;
    const binsPerBand = usefulBinCount / BAND_COUNT;

    // --- 256バンドの計算 ---
    const bands: number[] = new Array(BAND_COUNT);
    for (let b = 0; b < BAND_COUNT; b++) {
      const binStart = minBin + Math.floor(b * binsPerBand);
      const binEnd = minBin + Math.floor((b + 1) * binsPerBand);
      const count = binEnd - binStart;
      if (count <= 0) {
        bands[b] = 0;
        continue;
      }
      let sum = 0;
      for (let i = binStart; i < binEnd; i++) sum += dataArray[i];
      bands[b] = (sum / count) / 255.0;
    }

    // --- bass / mid / high の計算（周波数ベースで境界を決定）---
    // bands は MIN_FREQ_HZ ~ MAX_FREQ_HZ を均等分割するので、
    // バンドインデックスへの変換: bandIndex = (freq - MIN_FREQ_HZ) / (MAX_FREQ_HZ - MIN_FREQ_HZ) * BAND_COUNT
    const freqRange = MAX_FREQ_HZ - MIN_FREQ_HZ;
    const bassEndBand = Math.round((BASS_MAX_HZ - MIN_FREQ_HZ) / freqRange * BAND_COUNT);   // ~3
    const midEndBand = Math.round((MID_MAX_HZ - MIN_FREQ_HZ) / freqRange * BAND_COUNT);     // ~66

    const sum = (arr: number[], from: number, to: number) => {
      let s = 0;
      for (let i = from; i < to; i++) s += arr[i];
      return s;
    };

    const audioData: AudioData = {
      volume: bands.reduce((a, b) => a + b, 0) / BAND_COUNT,
      bass: bassEndBand > 0
        ? sum(bands, 0, bassEndBand) / bassEndBand
        : 0,
      mid: midEndBand > bassEndBand
        ? sum(bands, bassEndBand, midEndBand) / (midEndBand - bassEndBand)
        : 0,
      high: BAND_COUNT > midEndBand
        ? sum(bands, midEndBand, BAND_COUNT) / (BAND_COUNT - midEndBand)
        : 0,
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
      analyser.smoothingTimeConstant = 0.8;
      analyserRef.current = analyser;
      dataArrayRef.current = new Uint8Array(analyser.frequencyBinCount);

      const source = ctx.createMediaStreamSource(stream);
      source.connect(analyser);
      sourceRef.current = source;

      setIsActive(true);
      analyzeAndEmit();
    } catch (err: any) {
      console.error('Failed to start audio analyzer:', err);
      setError(`マイクの初期化に失敗しました: ${err.message || err}`);
      cleanup();
    }
  }, [analyzeAndEmit, cleanup]);

  const stop = useCallback(() => {
    cleanup();
  }, [cleanup]);

  useEffect(() => {
    return () => {
      cleanup();
    };
  }, [cleanup]);

  return { isActive, start, stop, error };
}
