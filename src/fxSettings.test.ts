import { describe, expect, it } from "vitest";
import {
  DEFAULT_FX_SETTINGS,
  createFxRuntimePatch,
  createFxSettingsChangedPayload,
  haveFxRuntimeValuesChanged,
  isFxChangeForSketch,
  mergeFxSettings,
  shouldApplyFxSettings,
  updateSketchFxSettings,
  type FxRuntimeValues,
} from "./fxSettings";

const runtimeValues: FxRuntimeValues = {
  strength: 0.5,
  radius: 0.25,
  threshold: 0.75,
  rgbAmount: 0.01,
  filmIntensity: 0.2,
  vignetteOffset: 0.3,
  vignetteDarkness: 0.8,
};

describe("FX settings ownership", () => {
  it("merges nested partial settings without changing untouched values", () => {
    const merged = mergeFxSettings(DEFAULT_FX_SETTINGS, {
      bloom: { strength: 0.5 },
      vignette: { darkness: 0.4 },
    });

    expect(merged).toEqual({
      bloom: { strength: 0.5, radius: 0, threshold: 1 },
      rgbShift: { amount: 0 },
      film: { intensity: 0 },
      vignette: { offset: 0, darkness: 0.4 },
    });
  });

  it("updates only the selected sketch and preserves the previous map", () => {
    const firstSettings = mergeFxSettings(DEFAULT_FX_SETTINGS, { film: { intensity: 0.2 } });
    const original = { "first.js": firstSettings };

    const updated = updateSketchFxSettings(original, "second.js", {
      rgbShift: { amount: 0.03 },
    });

    expect(updated).not.toBe(original);
    expect(updated["first.js"]).toBe(firstSettings);
    expect(updated["second.js"]).toEqual({
      ...DEFAULT_FX_SETTINGS,
      rgbShift: { amount: 0.03 },
    });
  });

  it("accepts host sync only for the current named sketch", () => {
    expect(isFxChangeForSketch("first.js", "first.js")).toBe(true);
    expect(isFxChangeForSketch("second.js", "first.js")).toBe(false);
    expect(isFxChangeForSketch(undefined, "first.js")).toBe(false);
    expect(isFxChangeForSketch("first.js", null)).toBe(false);
  });

  it("applies unscoped or active-sketch settings in the visualizer", () => {
    expect(shouldApplyFxSettings(undefined, "first.js")).toBe(true);
    expect(shouldApplyFxSettings("first.js", "first.js")).toBe(true);
    expect(shouldApplyFxSettings("second.js", "first.js")).toBe(false);
    expect(shouldApplyFxSettings("first.js", null)).toBe(false);
  });
});

describe("FX synchronization", () => {
  it("maps partial nested settings to runtime values without dropping zeroes", () => {
    expect(createFxRuntimePatch({
      bloom: { strength: 0, threshold: 0.75 },
      rgbShift: { amount: 0.01 },
      vignette: { darkness: 0 },
    })).toEqual({
      strength: 0,
      threshold: 0.75,
      rgbAmount: 0.01,
      vignetteDarkness: 0,
    });
  });

  it("uses the existing per-property change thresholds", () => {
    expect(haveFxRuntimeValuesChanged(
      { ...runtimeValues, strength: runtimeValues.strength + 0.0009 },
      runtimeValues,
    )).toBe(false);
    expect(haveFxRuntimeValuesChanged(
      { ...runtimeValues, strength: runtimeValues.strength + 0.0011 },
      runtimeValues,
    )).toBe(true);
    expect(haveFxRuntimeValuesChanged(
      { ...runtimeValues, rgbAmount: runtimeValues.rgbAmount + 0.00009 },
      runtimeValues,
    )).toBe(false);
    expect(haveFxRuntimeValuesChanged(
      { ...runtimeValues, rgbAmount: runtimeValues.rgbAmount + 0.00011 },
      runtimeValues,
    )).toBe(true);
  });

  it("does not report identical runtime values as changed", () => {
    expect(haveFxRuntimeValuesChanged({ ...runtimeValues }, runtimeValues)).toBe(false);
  });

  it("creates the existing nested host event payload", () => {
    expect(createFxSettingsChangedPayload("first.js", runtimeValues)).toEqual({
      sketchPath: "first.js",
      bloom: { strength: 0.5, radius: 0.25, threshold: 0.75 },
      rgbShift: { amount: 0.01 },
      film: { intensity: 0.2 },
      vignette: { offset: 0.3, darkness: 0.8 },
    });
    expect(createFxSettingsChangedPayload(null, runtimeValues).sketchPath).toBeUndefined();
  });
});
