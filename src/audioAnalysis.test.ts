import { describe, expect, it } from "vitest";
import {
  analyzeFrequencyData,
  downsampleWaveform,
  hasWaveformSignal,
  normalizeWaveformChannels,
  type AudioFrequencyConfig,
  type WaveformChannels,
} from "./audioAnalysis";

const frequencyConfig: AudioFrequencyConfig = {
  fftSize: 4096,
  bandCount: 256,
  minFreq: 27.5,
  maxFreq: 4186,
  bassMaxHz: 250,
  midMaxHz: 2000,
};

const waveform = (
  mono: number[],
  left: number[],
  right: number[],
): WaveformChannels => ({
  mono: new Float32Array(mono),
  left: new Float32Array(left),
  right: new Float32Array(right),
});

describe("hasWaveformSignal", () => {
  it("detects samples at either threshold boundary", () => {
    expect(hasWaveformSignal(new Float32Array([0, 0.001]))).toBe(true);
    expect(hasWaveformSignal(new Float32Array([0, -0.001]))).toBe(true);
  });

  it("treats sub-threshold samples as silence", () => {
    expect(hasWaveformSignal(new Float32Array([0.0009, -0.0009]))).toBe(false);
  });
});

describe("normalizeWaveformChannels", () => {
  it("mirrors mono samples for an explicitly mono input", () => {
    const channels = waveform([0.25, -0.5], [0, 0], [0.8, 0.8]);

    normalizeWaveformChannels(channels, true, 1);

    expect(Array.from(channels.left)).toEqual([0.25, -0.5]);
    expect(Array.from(channels.right)).toEqual([0.25, -0.5]);
  });

  it("mirrors mono when channel metadata is absent and the right channel is silent", () => {
    const channels = waveform([0.25, -0.5], [0.1, 0.2], [0, 0]);

    normalizeWaveformChannels(channels, false, undefined);

    expect(Array.from(channels.left)).toEqual([0.25, -0.5]);
    expect(Array.from(channels.right)).toEqual([0.25, -0.5]);
  });

  it("keeps unknown-channel stereo data when the right channel has a signal", () => {
    const channels = waveform([0.25, -0.5], [0.1, 0.2], [0.3, 0.4]);

    normalizeWaveformChannels(channels, false, undefined);

    expect(Array.from(channels.left)).toEqual(expect.arrayContaining([
      expect.closeTo(0.1),
      expect.closeTo(0.2),
    ]));
    expect(Array.from(channels.right)).toEqual(expect.arrayContaining([
      expect.closeTo(0.3),
      expect.closeTo(0.4),
    ]));
  });

  it("keeps an intentionally silent right channel for reported stereo input", () => {
    const channels = waveform([0.25, -0.5], [0.1, 0.2], [0, 0]);

    normalizeWaveformChannels(channels, false, 2);

    expect(Array.from(channels.left)).toEqual(expect.arrayContaining([
      expect.closeTo(0.1),
      expect.closeTo(0.2),
    ]));
    expect(Array.from(channels.right)).toEqual([0, 0]);
  });
});

describe("downsampleWaveform", () => {
  it("stores the minimum and maximum sample in each bucket", () => {
    const target = { min: new Array<number>(2), max: new Array<number>(2) };

    downsampleWaveform(new Float32Array([-0.5, 0.25, -0.25, 0.75]), target);

    expect(target.min).toEqual([-0.5, -0.25]);
    expect(target.max).toEqual([0.25, 0.75]);
  });

  it("updates existing target arrays without replacing them", () => {
    const min = [0, 0];
    const max = [0, 0];
    const target = { min, max };

    downsampleWaveform(new Float32Array([0.1, 0.2, 0.3, 0.4]), target);

    expect(target.min).toBe(min);
    expect(target.max).toBe(max);
  });
});

describe("analyzeFrequencyData", () => {
  it("returns zeroed, finite output for silence", () => {
    const result = analyzeFrequencyData(new Uint8Array(2048), 48_000, frequencyConfig);

    expect(result.bands).toHaveLength(256);
    expect(result.bands.every((value) => value === 0)).toBe(true);
    expect(result).toMatchObject({ volume: 0, bass: 0, mid: 0, high: 0 });
  });

  it("returns fully saturated output for maximum input", () => {
    const result = analyzeFrequencyData(
      new Uint8Array(2048).fill(255),
      48_000,
      frequencyConfig,
    );

    expect(result.bands.every((value) => value === 1)).toBe(true);
    expect(result).toMatchObject({ volume: 1, bass: 1, mid: 1, high: 1 });
  });

  it("applies the existing high-frequency tilt and clamps every band", () => {
    const result = analyzeFrequencyData(
      new Uint8Array(2048).fill(128),
      48_000,
      frequencyConfig,
    );

    expect(result.bands[0]).toBeCloseTo(Math.pow(128 / 255, 1.5));
    expect(result.bands[result.bands.length - 1]).toBeGreaterThan(result.bands[0]);
    expect(result.bands.every((value) => value >= 0 && value <= 1)).toBe(true);
    expect([result.volume, result.bass, result.mid, result.high].every(Number.isFinite)).toBe(true);
  });
});
