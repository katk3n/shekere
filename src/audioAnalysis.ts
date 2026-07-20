export interface AudioFrequencySummary {
  volume: number;
  bass: number;
  mid: number;
  high: number;
  bands: number[];
}

export interface AudioFrequencyConfig {
  fftSize: number;
  bandCount: number;
  minFreq: number;
  maxFreq: number;
  bassMaxHz: number;
  midMaxHz: number;
}

export interface WaveformChannels {
  mono: Float32Array;
  left: Float32Array;
  right: Float32Array;
}

export interface WaveformPreviewChannel {
  min: number[];
  max: number[];
}

export function hasWaveformSignal(samples: Float32Array, threshold = 0.001): boolean {
  for (let index = 0; index < samples.length; index++) {
    if (Math.abs(samples[index]) >= threshold) return true;
  }
  return false;
}

export function normalizeWaveformChannels(
  waveform: WaveformChannels,
  isMonoInput: boolean,
  reportedInputChannelCount: number | undefined,
): void {
  const shouldMirrorMono = isMonoInput || (
    reportedInputChannelCount === undefined &&
    hasWaveformSignal(waveform.mono) &&
    !hasWaveformSignal(waveform.right)
  );

  if (shouldMirrorMono) {
    waveform.left.set(waveform.mono);
    waveform.right.set(waveform.mono);
  }
}

export function downsampleWaveform(
  source: Float32Array,
  target: WaveformPreviewChannel,
): void {
  const bucketCount = target.min.length;
  for (let bucket = 0; bucket < bucketCount; bucket++) {
    const start = Math.floor((bucket * source.length) / bucketCount);
    const end = Math.floor(((bucket + 1) * source.length) / bucketCount);
    let min = 1;
    let max = -1;

    for (let sample = start; sample < end; sample++) {
      const value = source[sample];
      if (value < min) min = value;
      if (value > max) max = value;
    }

    target.min[bucket] = min;
    target.max[bucket] = max;
  }
}

export function analyzeFrequencyData(
  frequencyData: Uint8Array,
  sampleRate: number,
  config: AudioFrequencyConfig,
): AudioFrequencySummary {
  const {
    fftSize,
    bandCount,
    minFreq,
    maxFreq,
    bassMaxHz,
    midMaxHz,
  } = config;
  const binResolution = sampleRate / fftSize;
  const logRatio = Math.pow(maxFreq / minFreq, 1 / bandCount);
  const bands = new Array<number>(bandCount);

  for (let band = 0; band < bandCount; band++) {
    const freqStart = minFreq * Math.pow(logRatio, band);
    const freqEnd = minFreq * Math.pow(logRatio, band + 1);
    const binStart = Math.floor(freqStart / binResolution);
    const binEnd = Math.max(binStart + 1, Math.floor(freqEnd / binResolution));

    let sum = 0;
    let count = 0;
    for (let bin = binStart; bin < binEnd && bin < frequencyData.length; bin++) {
      sum += frequencyData[bin];
      count++;
    }

    let value = count > 0 ? (sum / count) / 255 : 0;
    value *= 1 + (band / bandCount) * 0.8;
    bands[band] = Math.min(1, Math.pow(value, 1.5));
  }

  const getBandIndex = (frequency: number): number => {
    if (frequency <= minFreq) return 0;
    if (frequency >= maxFreq) return bandCount;
    return Math.floor(Math.log(frequency / minFreq) / Math.log(logRatio));
  };

  const averageRange = (from: number, to: number): number => {
    const start = Math.max(0, from);
    const end = Math.min(bands.length, to);
    if (start >= end) return 0;

    let sum = 0;
    for (let index = start; index < end; index++) sum += bands[index];
    return sum / (end - start);
  };

  const bassEnd = getBandIndex(bassMaxHz);
  const midEnd = getBandIndex(midMaxHz);

  return {
    volume: bands.reduce((sum, value) => sum + value, 0) / bandCount,
    bass: averageRange(0, bassEnd),
    mid: averageRange(bassEnd, midEnd),
    high: averageRange(midEnd, bandCount),
    bands,
  };
}
