import { describe, expect, it, vi } from "vitest";
import type * as THREE from "three";
import {
  CameraManager,
  classifyCameraError,
  type CameraStatus,
} from "./cameraManager";

class FakeTrack extends EventTarget {
  stop = vi.fn();

  getSettings(): MediaTrackSettings {
    return { width: 1280, height: 720, frameRate: 30 };
  }
}

function createStream(track = new FakeTrack()) {
  return {
    track,
    stream: {
      getTracks: () => [track],
      getVideoTracks: () => [track],
    } as unknown as MediaStream,
  };
}

function createVideo() {
  return {
    srcObject: null,
    videoWidth: 0,
    videoHeight: 0,
    play: vi.fn().mockResolvedValue(undefined),
    pause: vi.fn(),
    removeAttribute: vi.fn(),
    load: vi.fn(),
  } as unknown as HTMLVideoElement;
}

function createTexture() {
  return { dispose: vi.fn() } as unknown as THREE.VideoTexture;
}

function createMediaDevices(getUserMedia: MediaDevices["getUserMedia"]) {
  return {
    getUserMedia,
    enumerateDevices: vi.fn().mockResolvedValue([
      { kind: "videoinput", deviceId: "camera-1", label: "Camera 1" },
      { kind: "videoinput", deviceId: "camera-2", label: "Camera 2" },
    ]),
  } as unknown as MediaDevices;
}

describe("CameraManager", () => {
  it("keeps CameraData identity stable while replacing its texture", async () => {
    const first = createStream();
    const second = createStream();
    const getUserMedia = vi.fn()
      .mockResolvedValueOnce(first.stream)
      .mockResolvedValueOnce(second.stream);
    const textures = [createTexture(), createTexture()];
    const manager = new CameraManager({
      mediaDevices: createMediaDevices(getUserMedia),
      createVideoElement: createVideo,
      createVideoTexture: vi.fn()
        .mockReturnValueOnce(textures[0])
        .mockReturnValueOnce(textures[1]),
    });
    const data = manager.data;

    await manager.start();
    expect(manager.data).toBe(data);
    expect(data).toMatchObject({ active: true, width: 1280, height: 720, frameRate: 30 });
    expect(data.texture).toBe(textures[0]);

    await manager.selectDevice("camera-2");
    expect(manager.data).toBe(data);
    expect(data.texture).toBe(textures[1]);
    expect(first.track.stop).toHaveBeenCalledOnce();
    expect(textures[0].dispose).toHaveBeenCalledOnce();
    expect(getUserMedia.mock.calls[1][0]).toMatchObject({
      audio: false,
      video: {
        width: { ideal: 1280 },
        height: { ideal: 720 },
        frameRate: { ideal: 30 },
        deviceId: { exact: "camera-2" },
      },
    });
  });

  it("stops a stale stream that resolves after a newer request", async () => {
    const first = createStream();
    const second = createStream();
    let resolveFirst: ((stream: MediaStream) => void) | undefined;
    const firstRequest = new Promise<MediaStream>((resolve) => { resolveFirst = resolve; });
    const getUserMedia = vi.fn()
      .mockReturnValueOnce(firstRequest)
      .mockResolvedValueOnce(second.stream);
    const manager = new CameraManager({
      mediaDevices: createMediaDevices(getUserMedia),
      createVideoElement: createVideo,
      createVideoTexture: createTexture,
    });

    const staleStart = manager.start();
    await manager.selectDevice("camera-2");
    resolveFirst?.(first.stream);
    await staleStart;

    expect(first.track.stop).toHaveBeenCalledOnce();
    expect(second.track.stop).not.toHaveBeenCalled();
    expect(manager.data.active).toBe(true);
  });

  it("clears and disposes active resources on stop", async () => {
    const { stream, track } = createStream();
    const video = createVideo();
    const texture = createTexture();
    const statuses: CameraStatus[] = [];
    const manager = new CameraManager({
      mediaDevices: createMediaDevices(vi.fn().mockResolvedValue(stream)),
      createVideoElement: () => video,
      createVideoTexture: () => texture,
      onStatus: (status) => statuses.push(status),
    });

    await manager.start();
    manager.stop();

    expect(track.stop).toHaveBeenCalledOnce();
    expect(texture.dispose).toHaveBeenCalledOnce();
    expect(video.srcObject).toBeNull();
    expect(manager.data).toEqual({
      active: false,
      texture: null,
      width: 0,
      height: 0,
      frameRate: 0,
    });
    expect(statuses[statuses.length - 1]?.state).toBe("inactive");
  });

  it("moves to a safe error state when the active track ends", async () => {
    const { stream, track } = createStream();
    const manager = new CameraManager({
      mediaDevices: createMediaDevices(vi.fn().mockResolvedValue(stream)),
      createVideoElement: createVideo,
      createVideoTexture: createTexture,
    });

    await manager.start();
    track.dispatchEvent(new Event("ended"));

    expect(manager.data.active).toBe(false);
    expect(manager.getStatus()).toMatchObject({
      state: "error",
      error: { code: "device-unavailable" },
    });
  });
});

describe("classifyCameraError", () => {
  it.each([
    ["NotAllowedError", "", "permission-denied"],
    ["NotFoundError", "", "no-device"],
    ["NotFoundError", "camera-1", "device-unavailable"],
    ["OverconstrainedError", "camera-1", "constraints-unsupported"],
    ["NotReadableError", "camera-1", "capture-failed"],
  ])("maps %s to %s", (name, deviceId, code) => {
    expect(classifyCameraError({ name }, deviceId).code).toBe(code);
  });

  it("treats a deviceId constraint failure as an unavailable selected device", () => {
    expect(classifyCameraError(
      { name: "OverconstrainedError", constraint: "deviceId" },
      "camera-1",
    ).code).toBe("device-unavailable");
  });
});
