import type { WatchEvent } from "@tauri-apps/plugin-fs";

interface UserCodeUpdatePayload<TFxSettings> {
  code: string;
  dir: string;
  sketchPath: string;
  fxSettings: TFxSettings;
}

interface SketchFileWatcherOptions<TFxSettings> {
  sketchPath: string;
  fxSettings: TFxSettings;
  readFile: (path: string) => Promise<string>;
  emitUpdate: (payload: UserCodeUpdatePayload<TFxSettings>) => Promise<void>;
  watchFile: (
    path: string,
    callback: (event: WatchEvent) => void,
    options: { recursive: false; delayMs: number },
  ) => Promise<() => void>;
  onLoadSuccess: () => void;
  onLoadError: (error: unknown) => void;
  onWatchError: (error: unknown) => void;
  now?: () => number;
  throttleMs?: number;
}

export function isSketchModificationEvent(event: WatchEvent): boolean {
  return (
    event.type === "any" ||
    event.type === "other" ||
    (typeof event.type === "object" && "modify" in event.type)
  );
}

export function startSketchFileWatcher<TFxSettings>(
  options: SketchFileWatcherOptions<TFxSettings>,
): () => void {
  const now = options.now ?? Date.now;
  const throttleMs = options.throttleMs ?? 150;
  let disposed = false;
  let unwatch: (() => void) | null = null;
  let lastEmitTime = 0;

  const loadAndEmit = async (): Promise<void> => {
    try {
      const code = await options.readFile(options.sketchPath);
      if (disposed) return;

      const separatorIndex = Math.max(
        options.sketchPath.lastIndexOf("/"),
        options.sketchPath.lastIndexOf("\\"),
      );
      const dir = options.sketchPath.substring(0, separatorIndex + 1);
      await options.emitUpdate({
        code,
        dir,
        sketchPath: options.sketchPath,
        fxSettings: options.fxSettings,
      });
      if (!disposed) options.onLoadSuccess();
    } catch (error: unknown) {
      if (!disposed) options.onLoadError(error);
    }
  };

  void loadAndEmit();

  options.watchFile(
    options.sketchPath,
    (event) => {
      if (disposed || !isSketchModificationEvent(event)) return;

      const currentTime = now();
      if (currentTime - lastEmitTime > throttleMs) {
        lastEmitTime = currentTime;
        void loadAndEmit();
      }
    },
    { recursive: false, delayMs: 20 },
  ).then((unwatchFn) => {
    if (disposed) {
      unwatchFn();
    } else {
      unwatch = unwatchFn;
    }
  }).catch((error: unknown) => {
    if (!disposed) options.onWatchError(error);
  });

  return () => {
    disposed = true;
    unwatch?.();
    unwatch = null;
  };
}
