import { describe, expect, it, vi } from "vitest";
import {
  releaseAudioCapture,
  startAudioCapture,
  type AudioCaptureContext,
  type AudioCaptureStream,
} from "./audioCaptureLifecycle";

function createResources() {
  const stopFirst = vi.fn();
  const stopSecond = vi.fn();
  const close = vi.fn(async () => undefined);
  const stream: AudioCaptureStream = {
    getTracks: () => [{ stop: stopFirst }, { stop: stopSecond }],
  };
  const context: AudioCaptureContext = { close };

  return { close, context, stopFirst, stopSecond, stream };
}

describe("startAudioCapture", () => {
  it("returns initialized resources without releasing them", async () => {
    const resources = createResources();
    const initialize = vi.fn();

    const result = await startAudioCapture({
      acquireStream: async () => resources.stream,
      createContext: () => resources.context,
      initialize,
    });

    expect(result).toEqual({ stream: resources.stream, context: resources.context });
    expect(initialize).toHaveBeenCalledWith(resources.stream, resources.context);
    expect(resources.stopFirst).not.toHaveBeenCalled();
    expect(resources.close).not.toHaveBeenCalled();
  });

  it("propagates permission rejection without creating a context", async () => {
    const permissionError = new Error("permission denied");
    const createContext = vi.fn();

    await expect(startAudioCapture({
      acquireStream: async () => { throw permissionError; },
      createContext,
      initialize: vi.fn(),
    })).rejects.toBe(permissionError);

    expect(createContext).not.toHaveBeenCalled();
  });

  it("stops acquired tracks when context creation fails", async () => {
    const resources = createResources();
    const contextError = new Error("context failed");

    await expect(startAudioCapture({
      acquireStream: async () => resources.stream,
      createContext: () => { throw contextError; },
      initialize: vi.fn(),
    })).rejects.toBe(contextError);

    expect(resources.stopFirst).toHaveBeenCalledOnce();
    expect(resources.stopSecond).toHaveBeenCalledOnce();
    expect(resources.close).not.toHaveBeenCalled();
  });

  it("releases all resources when graph initialization fails", async () => {
    const resources = createResources();
    const initializationError = new Error("graph failed");

    await expect(startAudioCapture({
      acquireStream: async () => resources.stream,
      createContext: () => resources.context,
      initialize: () => { throw initializationError; },
    })).rejects.toBe(initializationError);

    expect(resources.stopFirst).toHaveBeenCalledOnce();
    expect(resources.stopSecond).toHaveBeenCalledOnce();
    expect(resources.close).toHaveBeenCalledOnce();
  });
});

describe("releaseAudioCapture", () => {
  it("stops every track and closes the audio context", async () => {
    const resources = createResources();

    await releaseAudioCapture(resources);

    expect(resources.stopFirst).toHaveBeenCalledOnce();
    expect(resources.stopSecond).toHaveBeenCalledOnce();
    expect(resources.close).toHaveBeenCalledOnce();
  });

  it("continues cleanup and reports individual release failures", async () => {
    const stopError = new Error("stop failed");
    const closeError = new Error("close failed");
    const stopSecond = vi.fn();
    const onError = vi.fn();

    await releaseAudioCapture({
      stream: {
        getTracks: () => [
          { stop: () => { throw stopError; } },
          { stop: stopSecond },
        ],
      },
      context: { close: async () => { throw closeError; } },
    }, onError);

    expect(stopSecond).toHaveBeenCalledOnce();
    expect(onError).toHaveBeenNthCalledWith(1, stopError);
    expect(onError).toHaveBeenNthCalledWith(2, closeError);
  });
});
