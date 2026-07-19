import { useCallback, useEffect, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import type { CameraDevice, CameraStatus } from "../cameraManager";

const INITIAL_STATUS: CameraStatus = {
  state: "inactive",
  active: false,
  selectedDeviceId: "",
  width: 0,
  height: 0,
  frameRate: 0,
  error: null,
};

export function useCameraControl() {
  const [devices, setDevices] = useState<CameraDevice[]>([]);
  const [status, setStatus] = useState<CameraStatus>(INITIAL_STATUS);

  useEffect(() => {
    const unlistenDevices = listen<{ devices: CameraDevice[] }>(
      "camera-device-list",
      (event) => setDevices(event.payload.devices),
    );
    const unlistenStatus = listen<CameraStatus>(
      "camera-status",
      (event) => setStatus(event.payload),
    );

    void Promise.all([unlistenDevices, unlistenStatus]).then(() => {
      void emit("request-camera-devices");
      void emit("request-camera-status");
    });

    return () => {
      void unlistenDevices.then((unlisten) => unlisten());
      void unlistenStatus.then((unlisten) => unlisten());
    };
  }, []);

  const start = useCallback(() => emit("start-camera"), []);
  const stop = useCallback(() => emit("stop-camera"), []);
  const selectDevice = useCallback(
    (deviceId: string) => emit("update-camera-device", { deviceId }),
    [],
  );

  return { devices, status, start, stop, selectDevice };
}
