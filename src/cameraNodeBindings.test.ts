import { describe, expect, it, vi } from "vitest";
import * as THREE from "three";
import { CameraNodeBindings } from "./cameraNodeBindings";

describe("CameraNodeBindings", () => {
  it("keeps a stable node while camera textures change", () => {
    const bindings = new CameraNodeBindings();
    const node = bindings.textureNode;
    const fallback = node.value;
    const first = {} as THREE.VideoTexture;
    const second = {} as THREE.VideoTexture;

    expect(fallback.name).toBe("Camera.fallback");
    expect(fallback.colorSpace).toBe(THREE.SRGBColorSpace);

    bindings.update({ active: true, texture: first });
    expect(bindings.textureNode).toBe(node);
    expect(node.value).toBe(first);

    bindings.update({ active: true, texture: second });
    expect(bindings.textureNode).toBe(node);
    expect(node.value).toBe(second);

    bindings.update({ active: false, texture: null });
    expect(bindings.textureNode).toBe(node);
    expect(node.value).toBe(fallback);
  });

  it("uses the fallback for inconsistent inactive input", () => {
    const bindings = new CameraNodeBindings();
    const fallback = bindings.textureNode.value;

    bindings.update({ active: false, texture: {} as THREE.VideoTexture });

    expect(bindings.textureNode.value).toBe(fallback);
  });

  it("disposes the fallback exactly once", () => {
    const bindings = new CameraNodeBindings();
    const fallback = bindings.textureNode.value;
    const dispose = vi.spyOn(fallback, "dispose");

    bindings.dispose();
    bindings.dispose();

    expect(dispose).toHaveBeenCalledOnce();
  });
});
