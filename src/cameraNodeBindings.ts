import * as THREE from "three";
import * as TSL from "three/tsl";

interface CameraTextureInput {
  active: boolean;
  texture: THREE.VideoTexture | null;
}

export class CameraNodeBindings {
  readonly textureNode: ReturnType<typeof TSL.texture>;
  private readonly fallbackTexture: THREE.DataTexture;
  private disposed = false;

  constructor() {
    this.fallbackTexture = new THREE.DataTexture(
      new Uint8Array([0, 0, 0, 255]),
      1,
      1,
      THREE.RGBAFormat,
    );
    this.fallbackTexture.name = "Camera.fallback";
    // Match VideoTexture color semantics so the TSL graph compiles with the
    // same color conversion before and after live capture starts.
    this.fallbackTexture.colorSpace = THREE.SRGBColorSpace;
    this.fallbackTexture.magFilter = THREE.LinearFilter;
    this.fallbackTexture.minFilter = THREE.LinearFilter;
    this.fallbackTexture.generateMipmaps = false;
    this.fallbackTexture.needsUpdate = true;
    this.textureNode = TSL.texture(this.fallbackTexture);
  }

  update(camera: CameraTextureInput): void {
    if (this.disposed) return;
    this.textureNode.value = camera.active && camera.texture
      ? camera.texture
      : this.fallbackTexture;
  }

  dispose(): void {
    if (this.disposed) return;
    this.textureNode.value = this.fallbackTexture;
    this.fallbackTexture.dispose();
    this.disposed = true;
  }
}
