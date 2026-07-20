import { describe, expect, it, vi } from "vitest";
import type { WatchEvent } from "@tauri-apps/plugin-fs";
import {
  isSketchModificationEvent,
  startSketchFileWatcher,
} from "./sketchFileWatcher";

const event = (type: WatchEvent["type"]): WatchEvent => ({
  type,
  paths: [],
  attrs: null,
});

function createHarness() {
  let watchCallback: ((event: WatchEvent) => void) | undefined;
  const unwatch = vi.fn();
  const readFile = vi.fn(async () => "export function setup() {}");
  const emitUpdate = vi.fn(async () => undefined);
  const watchFile = vi.fn(async (
    _path: string,
    callback: (event: WatchEvent) => void,
  ) => {
    watchCallback = callback;
    return unwatch;
  });
  const onLoadSuccess = vi.fn();
  const onLoadError = vi.fn();
  const onWatchError = vi.fn();
  let currentTime = 1_000;

  const dispose = startSketchFileWatcher({
    sketchPath: "/shows/current/sketch.js",
    fxSettings: { bloom: 0.5 },
    readFile,
    emitUpdate,
    watchFile,
    onLoadSuccess,
    onLoadError,
    onWatchError,
    now: () => currentTime,
  });

  return {
    dispose,
    emitUpdate,
    onLoadError,
    onLoadSuccess,
    onWatchError,
    readFile,
    setTime: (value: number) => { currentTime = value; },
    trigger: (watchEvent: WatchEvent) => watchCallback?.(watchEvent),
    unwatch,
    watchFile,
  };
}

describe("isSketchModificationEvent", () => {
  it("accepts any, other, and modify events", () => {
    expect(isSketchModificationEvent(event("any"))).toBe(true);
    expect(isSketchModificationEvent(event("other"))).toBe(true);
    expect(isSketchModificationEvent(event({ modify: { kind: "any" } }))).toBe(true);
  });

  it("ignores unrelated filesystem events", () => {
    expect(isSketchModificationEvent(event({ create: { kind: "file" } }))).toBe(false);
    expect(isSketchModificationEvent(event({ remove: { kind: "file" } }))).toBe(false);
  });
});

describe("startSketchFileWatcher", () => {
  it("loads immediately and emits the existing update payload", async () => {
    const harness = createHarness();

    await vi.waitFor(() => expect(harness.emitUpdate).toHaveBeenCalledOnce());

    expect(harness.readFile).toHaveBeenCalledWith("/shows/current/sketch.js");
    expect(harness.emitUpdate).toHaveBeenCalledWith({
      code: "export function setup() {}",
      dir: "/shows/current/",
      sketchPath: "/shows/current/sketch.js",
      fxSettings: { bloom: 0.5 },
    });
    expect(harness.onLoadSuccess).toHaveBeenCalledOnce();
  });

  it("throttles accepted filesystem events", async () => {
    const harness = createHarness();
    await vi.waitFor(() => expect(harness.readFile).toHaveBeenCalledTimes(1));

    harness.trigger(event({ modify: { kind: "any" } }));
    await vi.waitFor(() => expect(harness.readFile).toHaveBeenCalledTimes(2));

    harness.setTime(1_100);
    harness.trigger(event("any"));
    await Promise.resolve();
    expect(harness.readFile).toHaveBeenCalledTimes(2);

    harness.setTime(1_151);
    harness.trigger(event("other"));
    await vi.waitFor(() => expect(harness.readFile).toHaveBeenCalledTimes(3));
  });

  it("unwatches an active watcher during cleanup", async () => {
    const harness = createHarness();
    await vi.waitFor(() => expect(harness.watchFile).toHaveResolved());

    harness.dispose();

    expect(harness.unwatch).toHaveBeenCalledOnce();
  });

  it("unwatches when registration completes after cleanup", async () => {
    let resolveWatch: ((unwatch: () => void) => void) | undefined;
    const unwatch = vi.fn();
    const watchPromise = new Promise<() => void>((resolve) => { resolveWatch = resolve; });
    const watchFile = vi.fn(() => watchPromise);
    const dispose = startSketchFileWatcher({
      sketchPath: "sketch.js",
      fxSettings: {},
      readFile: vi.fn(async () => "code"),
      emitUpdate: vi.fn(async () => undefined),
      watchFile,
      onLoadSuccess: vi.fn(),
      onLoadError: vi.fn(),
      onWatchError: vi.fn(),
    });

    dispose();
    resolveWatch?.(unwatch);
    await watchPromise;
    await vi.waitFor(() => expect(unwatch).toHaveBeenCalledOnce());
  });

  it("does not emit a stale read after cleanup", async () => {
    let resolveRead: ((code: string) => void) | undefined;
    const readPromise = new Promise<string>((resolve) => { resolveRead = resolve; });
    const emitUpdate = vi.fn(async () => undefined);
    const dispose = startSketchFileWatcher({
      sketchPath: "sketch.js",
      fxSettings: {},
      readFile: vi.fn(() => readPromise),
      emitUpdate,
      watchFile: vi.fn(async () => vi.fn()),
      onLoadSuccess: vi.fn(),
      onLoadError: vi.fn(),
      onWatchError: vi.fn(),
    });

    dispose();
    resolveRead?.("stale code");
    await readPromise;
    await Promise.resolve();

    expect(emitUpdate).not.toHaveBeenCalled();
  });

  it("reports active read and watch failures", async () => {
    const readError = new Error("read failed");
    const watchError = new Error("watch failed");
    const onLoadError = vi.fn();
    const onWatchError = vi.fn();

    startSketchFileWatcher({
      sketchPath: "sketch.js",
      fxSettings: {},
      readFile: vi.fn(async () => { throw readError; }),
      emitUpdate: vi.fn(async () => undefined),
      watchFile: vi.fn(async () => { throw watchError; }),
      onLoadSuccess: vi.fn(),
      onLoadError,
      onWatchError,
    });

    await vi.waitFor(() => expect(onLoadError).toHaveBeenCalledWith(readError));
    expect(onWatchError).toHaveBeenCalledWith(watchError);
  });
});
