import * as THREE from "three";

export interface CameraData {
  active: boolean;
  texture: THREE.VideoTexture | null;
  width: number;
  height: number;
  frameRate: number;
  motion: CameraMotionData;
}

export interface CameraMotionData {
  active: boolean;
  maskTexture: THREE.Texture | null;
  trailTexture: THREE.Texture | null;
  width: number;
  height: number;
}

export type CameraState = "inactive" | "starting" | "active" | "error";

export type CameraErrorCode =
  | "permission-denied"
  | "no-device"
  | "device-unavailable"
  | "constraints-unsupported"
  | "api-unavailable"
  | "playback-failed"
  | "capture-failed";

export interface CameraError {
  code: CameraErrorCode;
  message: string;
}

export interface CameraStatus {
  state: CameraState;
  active: boolean;
  selectedDeviceId: string;
  width: number;
  height: number;
  frameRate: number;
  error: CameraError | null;
}

export interface CameraDevice {
  deviceId: string;
  label: string;
}

interface CameraManagerOptions {
  mediaDevices?: MediaDevices;
  createVideoElement?: () => HTMLVideoElement;
  createVideoTexture?: (video: HTMLVideoElement) => THREE.VideoTexture;
  onStatus?: (status: CameraStatus) => void;
  onDevices?: (devices: CameraDevice[]) => void;
}

const CAMERA_CONSTRAINTS = {
  width: { ideal: 1280 },
  height: { ideal: 720 },
  frameRate: { ideal: 30 },
} satisfies MediaTrackConstraints;

function stopStream(stream: MediaStream) {
  stream.getTracks().forEach((track) => track.stop());
}

function clearVideo(video: HTMLVideoElement) {
  video.pause();
  video.srcObject = null;
  video.removeAttribute("src");
  video.load();
}

function defaultCreateVideoElement() {
  const video = document.createElement("video");
  video.autoplay = true;
  video.muted = true;
  video.playsInline = true;
  return video;
}

function defaultCreateVideoTexture(video: HTMLVideoElement) {
  const texture = new THREE.VideoTexture(video);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

export function classifyCameraError(
  error: unknown,
  selectedDeviceId: string,
  playback = false,
): CameraError {
  if (playback) {
    return {
      code: "playback-failed",
      message: "The camera stream was opened, but video playback could not start.",
    };
  }

  const name = error instanceof DOMException
    ? error.name
    : typeof error === "object" && error !== null && "name" in error
      ? String(error.name)
      : "";

  if (name === "NotAllowedError" || name === "SecurityError") {
    return {
      code: "permission-denied",
      message: "Camera permission was denied. Allow camera access and try again.",
    };
  }
  if (name === "NotFoundError" || name === "DevicesNotFoundError") {
    return selectedDeviceId
      ? {
          code: "device-unavailable",
          message: "The selected camera is unavailable or has been disconnected.",
        }
      : {
          code: "no-device",
          message: "No camera device is available.",
        };
  }
  if (name === "OverconstrainedError" || name === "ConstraintNotSatisfiedError") {
    const constraint = typeof error === "object" && error !== null && "constraint" in error
      ? String(error.constraint)
      : "";
    if (selectedDeviceId && constraint === "deviceId") {
      return {
        code: "device-unavailable",
        message: "The selected camera is unavailable or has been disconnected.",
      };
    }
    return {
      code: "constraints-unsupported",
      message: "The selected camera does not support the requested capture constraints.",
    };
  }

  return {
    code: "capture-failed",
    message: "The camera could not be started. Check the device and try again.",
  };
}

export class CameraManager {
  readonly data: CameraData = {
    active: false,
    texture: null,
    width: 0,
    height: 0,
    frameRate: 0,
    motion: {
      active: false,
      maskTexture: null,
      trailTexture: null,
      width: 0,
      height: 0,
    },
  };

  private readonly mediaDevices: MediaDevices | undefined;
  private readonly createVideoElement: () => HTMLVideoElement;
  private readonly createVideoTexture: (video: HTMLVideoElement) => THREE.VideoTexture;
  private readonly onStatus: (status: CameraStatus) => void;
  private readonly onDevices: (devices: CameraDevice[]) => void;
  private selectedDeviceId = "";
  private state: CameraState = "inactive";
  private error: CameraError | null = null;
  private requestGeneration = 0;
  private stream: MediaStream | null = null;
  private video: HTMLVideoElement | null = null;
  private track: MediaStreamTrack | null = null;
  private trackEndedHandler: (() => void) | null = null;

  constructor(options: CameraManagerOptions = {}) {
    this.mediaDevices = options.mediaDevices
      ?? (typeof navigator !== "undefined" ? navigator.mediaDevices : undefined);
    this.createVideoElement = options.createVideoElement ?? defaultCreateVideoElement;
    this.createVideoTexture = options.createVideoTexture ?? defaultCreateVideoTexture;
    this.onStatus = options.onStatus ?? (() => undefined);
    this.onDevices = options.onDevices ?? (() => undefined);
  }

  getStatus(): CameraStatus {
    return {
      state: this.state,
      active: this.data.active,
      selectedDeviceId: this.selectedDeviceId,
      width: this.data.width,
      height: this.data.height,
      frameRate: this.data.frameRate,
      error: this.error,
    };
  }

  async refreshDevices(): Promise<CameraDevice[]> {
    if (!this.mediaDevices?.enumerateDevices) {
      this.fail({
        code: "api-unavailable",
        message: "Camera capture APIs are unavailable in this environment.",
      });
      this.onDevices([]);
      return [];
    }

    try {
      const devices = (await this.mediaDevices.enumerateDevices())
        .filter((device) => device.kind === "videoinput")
        .map((device, index) => ({
          deviceId: device.deviceId,
          label: device.label || `Camera ${index + 1}`,
        }));

      this.onDevices(devices);
      if (
        this.selectedDeviceId
        && !devices.some((device) => device.deviceId === this.selectedDeviceId)
      ) {
        this.requestGeneration += 1;
        this.releaseResources();
        this.fail({
          code: "device-unavailable",
          message: "The selected camera is unavailable or has been disconnected.",
        });
      }
      return devices;
    } catch (error) {
      this.fail(classifyCameraError(error, this.selectedDeviceId));
      this.onDevices([]);
      return [];
    }
  }

  async selectDevice(deviceId: string): Promise<void> {
    if (deviceId === this.selectedDeviceId) return;
    const shouldRestart = this.state === "active" || this.state === "starting";
    this.selectedDeviceId = deviceId;
    this.emitStatus();
    if (shouldRestart) await this.start();
  }

  async start(): Promise<void> {
    const generation = ++this.requestGeneration;
    this.releaseResources();

    if (!this.mediaDevices?.getUserMedia) {
      this.fail({
        code: "api-unavailable",
        message: "Camera capture APIs are unavailable in this environment.",
      });
      return;
    }

    this.state = "starting";
    this.error = null;
    this.emitStatus();

    let stream: MediaStream | null = null;
    let video: HTMLVideoElement | null = null;
    let texture: THREE.VideoTexture | null = null;
    try {
      stream = await this.mediaDevices.getUserMedia({
        audio: false,
        video: this.selectedDeviceId
          ? { ...CAMERA_CONSTRAINTS, deviceId: { exact: this.selectedDeviceId } }
          : CAMERA_CONSTRAINTS,
      });

      if (generation !== this.requestGeneration) {
        stopStream(stream);
        return;
      }

      const track = stream.getVideoTracks()[0];
      if (!track) {
        stopStream(stream);
        this.fail({ code: "no-device", message: "No camera video track is available." });
        return;
      }

      video = this.createVideoElement();
      video.srcObject = stream;
      try {
        await video.play();
      } catch (error) {
        stopStream(stream);
        clearVideo(video);
        if (generation === this.requestGeneration) {
          this.fail(classifyCameraError(error, this.selectedDeviceId, true));
        }
        return;
      }

      if (generation !== this.requestGeneration) {
        stopStream(stream);
        clearVideo(video);
        return;
      }

      texture = this.createVideoTexture(video);
      const settings = track.getSettings();
      this.stream = stream;
      this.video = video;
      this.track = track;
      this.trackEndedHandler = () => this.handleTrackEnded(track);
      track.addEventListener("ended", this.trackEndedHandler);

      this.data.active = true;
      this.data.texture = texture;
      this.data.width = settings.width ?? video.videoWidth ?? 0;
      this.data.height = settings.height ?? video.videoHeight ?? 0;
      this.data.frameRate = settings.frameRate ?? 0;
      this.state = "active";
      this.error = null;
      this.emitStatus();
      void this.refreshDevices();
    } catch (error) {
      texture?.dispose();
      if (video) clearVideo(video);
      if (stream) stopStream(stream);
      if (generation === this.requestGeneration) {
        this.fail(classifyCameraError(error, this.selectedDeviceId));
      }
    }
  }

  stop(): void {
    this.requestGeneration += 1;
    this.releaseResources();
    this.state = "inactive";
    this.error = null;
    this.emitStatus();
  }

  dispose(): void {
    this.stop();
  }

  private handleTrackEnded(endedTrack: MediaStreamTrack) {
    if (endedTrack !== this.track) return;
    this.requestGeneration += 1;
    this.releaseResources();
    this.fail({
      code: "device-unavailable",
      message: "The selected camera is unavailable or has been disconnected.",
    });
    void this.refreshDevices();
  }

  private releaseResources() {
    if (this.track && this.trackEndedHandler) {
      this.track.removeEventListener("ended", this.trackEndedHandler);
    }
    this.trackEndedHandler = null;
    this.track = null;

    if (this.stream) stopStream(this.stream);
    this.stream = null;

    if (this.video) {
      clearVideo(this.video);
    }
    this.video = null;

    this.data.texture?.dispose();
    this.data.active = false;
    this.data.texture = null;
    this.data.width = 0;
    this.data.height = 0;
    this.data.frameRate = 0;
  }

  private fail(error: CameraError) {
    this.releaseResources();
    this.state = "error";
    this.error = error;
    this.emitStatus();
  }

  private emitStatus() {
    this.onStatus(this.getStatus());
  }
}
