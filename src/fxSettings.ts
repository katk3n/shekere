export interface FxSettings {
  bloom: { strength: number; radius: number; threshold: number };
  rgbShift: { amount: number };
  film: { intensity: number };
  vignette: { offset: number; darkness: number };
}

export interface FxSettingsChange {
  sketchPath?: string;
  bloom?: Partial<FxSettings["bloom"]>;
  rgbShift?: Partial<FxSettings["rgbShift"]>;
  film?: Partial<FxSettings["film"]>;
  vignette?: Partial<FxSettings["vignette"]>;
}

export interface FxRuntimeValues {
  strength: number;
  radius: number;
  threshold: number;
  rgbAmount: number;
  filmIntensity: number;
  vignetteOffset: number;
  vignetteDarkness: number;
}

export const DEFAULT_FX_SETTINGS: FxSettings = {
  bloom: { strength: 0, radius: 0, threshold: 1 },
  rgbShift: { amount: 0 },
  film: { intensity: 0 },
  vignette: { offset: 0, darkness: 1 },
};

export function mergeFxSettings(current: FxSettings, changes: FxSettingsChange): FxSettings {
  return {
    bloom: { ...current.bloom, ...changes.bloom },
    rgbShift: { ...current.rgbShift, ...changes.rgbShift },
    film: { ...current.film, ...changes.film },
    vignette: { ...current.vignette, ...changes.vignette },
  };
}

export function updateSketchFxSettings(
  settingsBySketch: Readonly<Record<string, FxSettings>>,
  sketchPath: string,
  changes: FxSettingsChange,
): Record<string, FxSettings> {
  return {
    ...settingsBySketch,
    [sketchPath]: mergeFxSettings(
      settingsBySketch[sketchPath] ?? DEFAULT_FX_SETTINGS,
      changes,
    ),
  };
}

export function isFxChangeForSketch(
  payloadSketchPath: string | undefined,
  currentSketchPath: string | null | undefined,
): boolean {
  return Boolean(payloadSketchPath && payloadSketchPath === currentSketchPath);
}

export function shouldApplyFxSettings(
  payloadSketchPath: string | undefined,
  activeSketchPath: string | null,
): boolean {
  return !payloadSketchPath || payloadSketchPath === activeSketchPath;
}

export function createFxRuntimePatch(changes: FxSettingsChange): Partial<FxRuntimeValues> {
  const patch: Partial<FxRuntimeValues> = {};
  if (changes.bloom?.strength !== undefined) patch.strength = changes.bloom.strength;
  if (changes.bloom?.radius !== undefined) patch.radius = changes.bloom.radius;
  if (changes.bloom?.threshold !== undefined) patch.threshold = changes.bloom.threshold;
  if (changes.rgbShift?.amount !== undefined) patch.rgbAmount = changes.rgbShift.amount;
  if (changes.film?.intensity !== undefined) patch.filmIntensity = changes.film.intensity;
  if (changes.vignette?.offset !== undefined) patch.vignetteOffset = changes.vignette.offset;
  if (changes.vignette?.darkness !== undefined) patch.vignetteDarkness = changes.vignette.darkness;
  return patch;
}

export function haveFxRuntimeValuesChanged(
  current: FxRuntimeValues,
  previous: FxRuntimeValues,
): boolean {
  return (
    Math.abs(current.strength - previous.strength) > 0.001 ||
    Math.abs(current.radius - previous.radius) > 0.001 ||
    Math.abs(current.threshold - previous.threshold) > 0.001 ||
    Math.abs(current.rgbAmount - previous.rgbAmount) > 0.0001 ||
    Math.abs(current.filmIntensity - previous.filmIntensity) > 0.001 ||
    Math.abs(current.vignetteOffset - previous.vignetteOffset) > 0.001 ||
    Math.abs(current.vignetteDarkness - previous.vignetteDarkness) > 0.001
  );
}

export function createFxSettingsChangedPayload(
  sketchPath: string | null,
  values: FxRuntimeValues,
): FxSettingsChange {
  return {
    sketchPath: sketchPath ?? undefined,
    bloom: {
      strength: values.strength,
      radius: values.radius,
      threshold: values.threshold,
    },
    rgbShift: { amount: values.rgbAmount },
    film: { intensity: values.filmIntensity },
    vignette: {
      offset: values.vignetteOffset,
      darkness: values.vignetteDarkness,
    },
  };
}
