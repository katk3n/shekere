import * as THREE from "three";
import {
  NodeMaterial,
  QuadMesh,
  type Node,
  type Renderer,
  type UniformNode,
} from "three/webgpu";
import * as TSL from "three/tsl";
import type { CameraMotionData } from "./cameraManager";
import { withRendererState } from "./rendererState";

export interface CameraMotionConfig {
  enabled?: boolean;
  threshold?: number;
  blur?: number;
  decay?: number;
}

export interface ResolvedCameraMotionConfig {
  enabled: boolean;
  threshold: number;
  blur: number;
  decay: number;
}

interface CameraMotionInput {
  active: boolean;
  texture: THREE.VideoTexture | null;
  width: number;
  height: number;
}

export interface CameraMotionPipeline {
  readonly maskTexture: THREE.Texture;
  readonly trailTexture: THREE.Texture;
  initializeFrame(): void;
  analyzeFrame(config: Readonly<ResolvedCameraMotionConfig>): void;
  dispose(): void;
}

export interface CameraMotionNodes {
  readonly maskNode: ReturnType<typeof TSL.texture>;
  readonly trailNode: ReturnType<typeof TSL.texture>;
}

export type CameraMotionPipelineFactory = (
  texture: THREE.VideoTexture,
  width: number,
  height: number,
) => CameraMotionPipeline;

interface CameraMotionAnalyzerOptions {
  createPipeline?: CameraMotionPipelineFactory;
  onError?: (error: unknown) => void;
}

export const DEFAULT_CAMERA_MOTION_CONFIG: Readonly<ResolvedCameraMotionConfig> = {
  enabled: false,
  threshold: 0.08,
  blur: 6,
  decay: 0.94,
};

const MAX_BLUR_RADIUS = 20;

function finiteNumberOrDefault(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

export function resolveCameraMotionConfig(
  config?: CameraMotionConfig,
): ResolvedCameraMotionConfig {
  return {
    enabled: config?.enabled === true,
    threshold: Math.min(
      1,
      Math.max(0, finiteNumberOrDefault(config?.threshold, DEFAULT_CAMERA_MOTION_CONFIG.threshold)),
    ),
    blur: Math.min(
      MAX_BLUR_RADIUS,
      Math.max(0, finiteNumberOrDefault(config?.blur, DEFAULT_CAMERA_MOTION_CONFIG.blur)),
    ),
    decay: Math.min(
      0.999,
      Math.max(0, finiteNumberOrDefault(config?.decay, DEFAULT_CAMERA_MOTION_CONFIG.decay)),
    ),
  };
}

export function calculateMotionAnalysisSize(
  sourceWidth: number,
  sourceHeight: number,
): { width: number; height: number } {
  if (sourceWidth <= 0 || sourceHeight <= 0) return { width: 0, height: 0 };

  const scale = 320 / Math.max(sourceWidth, sourceHeight);
  return {
    width: Math.max(1, Math.round(sourceWidth * scale)),
    height: Math.max(1, Math.round(sourceHeight * scale)),
  };
}

function setDataInactive(data: CameraMotionData): void {
  data.active = false;
  data.maskTexture = null;
  data.trailTexture = null;
  data.width = 0;
  data.height = 0;
}

function getVideoFrameTime(texture: THREE.VideoTexture): number | null {
  const image: unknown = texture.image;
  if (typeof image !== "object" || image === null || !("currentTime" in image)) return null;
  const currentTime = Number(image.currentTime);
  return Number.isFinite(currentTime) ? currentTime : null;
}

function createDataRenderTarget(width: number, height: number, name: string): THREE.RenderTarget {
  const target = new THREE.RenderTarget(width, height, {
    depthBuffer: false,
    format: THREE.RedFormat,
    magFilter: THREE.LinearFilter,
    minFilter: THREE.LinearFilter,
    generateMipmaps: false,
    colorSpace: THREE.NoColorSpace,
  });
  target.texture.name = name;
  target.texture.colorSpace = THREE.NoColorSpace;
  target.texture.generateMipmaps = false;
  return target;
}

function luminance(color: Node<"vec3">): Node<"float"> {
  return TSL.dot(color, TSL.vec3(0.2126, 0.7152, 0.0722));
}

function createCopyMaterial(source: THREE.Texture): NodeMaterial {
  const sourceNode = TSL.texture(source);
  const material = new NodeMaterial();
  const value = source instanceof THREE.VideoTexture
    ? luminance(sourceNode.sample(TSL.uv()).rgb)
    : sourceNode.sample(TSL.uv()).r;
  material.fragmentNode = TSL.vec4(value, value, value, 1);
  material.name = "CameraMotion.copy";
  return material;
}

function createMaskMaterial(
  cameraTexture: THREE.VideoTexture,
  previousTexture: THREE.Texture,
  thresholdNode: UniformNode<"float", number>,
): NodeMaterial {
  const uvNode = TSL.uv();
  const currentLuminance = luminance(TSL.texture(cameraTexture).sample(uvNode).rgb);
  const previousLuminance = TSL.texture(previousTexture).sample(uvNode).r;
  const difference = TSL.abs(currentLuminance.sub(previousLuminance));
  const mask = TSL.smoothstep(thresholdNode, thresholdNode.add(0.04), difference);
  const material = new NodeMaterial();
  material.fragmentNode = TSL.vec4(mask, mask, mask, 1);
  material.name = "CameraMotion.mask";
  return material;
}

function createBlurMaterial(
  source: THREE.Texture,
  radiusNode: UniformNode<"float", number>,
  texelStep: THREE.Vector2,
  name: string,
): NodeMaterial {
  const uvNode = TSL.uv();
  const sourceNode = TSL.texture(source);
  const sigma = TSL.max(radiusNode.div(3), 0.0001);
  let weightedSum: Node<"float"> = sourceNode.sample(uvNode).r;
  let totalWeight: Node<"float"> = TSL.float(1);

  for (let offset = 1; offset <= MAX_BLUR_RADIUS; offset += 1) {
    const active = TSL.step(offset, radiusNode);
    const normalizedOffset = TSL.float(offset).div(sigma);
    const weight = TSL.exp(normalizedOffset.mul(normalizedOffset).mul(-0.5)).mul(active);
    const uvOffset = TSL.vec2(texelStep).mul(offset);
    const positive = sourceNode.sample(uvNode.add(uvOffset)).r;
    const negative = sourceNode.sample(uvNode.sub(uvOffset)).r;
    weightedSum = weightedSum.add(positive.add(negative).mul(weight));
    totalWeight = totalWeight.add(weight.mul(2));
  }

  const blurred = weightedSum.div(totalWeight);
  const material = new NodeMaterial();
  material.fragmentNode = TSL.vec4(blurred, blurred, blurred, 1);
  material.name = name;
  return material;
}

function createTrailMaterial(
  maskTexture: THREE.Texture,
  previousTrailNode: ReturnType<typeof TSL.texture>,
  decayNode: UniformNode<"float", number>,
): NodeMaterial {
  const uvNode = TSL.uv();
  const mask = TSL.texture(maskTexture).sample(uvNode).r;
  const previousTrail = previousTrailNode.sample(uvNode).r;
  const trail = TSL.max(mask, previousTrail.mul(decayNode));
  const material = new NodeMaterial();
  material.fragmentNode = TSL.vec4(trail, trail, trail, 1);
  material.name = "CameraMotion.trail";
  return material;
}

class GpuCameraMotionPipeline implements CameraMotionPipeline {
  readonly maskTexture: THREE.Texture;
  private readonly previousTarget: THREE.RenderTarget;
  private readonly rawMaskTarget: THREE.RenderTarget;
  private readonly blurTarget: THREE.RenderTarget;
  private readonly maskTarget: THREE.RenderTarget;
  private trailReadTarget: THREE.RenderTarget;
  private trailWriteTarget: THREE.RenderTarget;
  private readonly copyMaterial: NodeMaterial;
  private readonly maskMaterial: NodeMaterial;
  private readonly horizontalBlurMaterial: NodeMaterial;
  private readonly verticalBlurMaterial: NodeMaterial;
  private readonly trailInputNode: ReturnType<typeof TSL.texture>;
  private readonly trailMaterial: NodeMaterial;
  private readonly thresholdNode = TSL.uniform(DEFAULT_CAMERA_MOTION_CONFIG.threshold);
  private readonly radiusNode = TSL.uniform(DEFAULT_CAMERA_MOTION_CONFIG.blur);
  private readonly decayNode = TSL.uniform(DEFAULT_CAMERA_MOTION_CONFIG.decay);
  private readonly quad = new QuadMesh();
  private disposed = false;

  constructor(
    private readonly renderer: Renderer,
    cameraTexture: THREE.VideoTexture,
    readonly width: number,
    readonly height: number,
  ) {
    this.previousTarget = createDataRenderTarget(width, height, "CameraMotion.previous");
    this.rawMaskTarget = createDataRenderTarget(width, height, "CameraMotion.rawMask");
    this.blurTarget = createDataRenderTarget(width, height, "CameraMotion.blurHorizontal");
    this.maskTarget = createDataRenderTarget(width, height, "CameraMotion.mask");
    this.trailReadTarget = createDataRenderTarget(width, height, "CameraMotion.trailA");
    this.trailWriteTarget = createDataRenderTarget(width, height, "CameraMotion.trailB");
    this.maskTexture = this.maskTarget.texture;

    this.copyMaterial = createCopyMaterial(cameraTexture);
    this.maskMaterial = createMaskMaterial(cameraTexture, this.previousTarget.texture, this.thresholdNode);
    this.horizontalBlurMaterial = createBlurMaterial(
      this.rawMaskTarget.texture,
      this.radiusNode,
      new THREE.Vector2(1 / width, 0),
      "CameraMotion.blurHorizontal",
    );
    this.verticalBlurMaterial = createBlurMaterial(
      this.blurTarget.texture,
      this.radiusNode,
      new THREE.Vector2(0, 1 / height),
      "CameraMotion.blurVertical",
    );
    this.trailInputNode = TSL.texture(this.trailReadTarget.texture);
    this.trailMaterial = createTrailMaterial(
      this.maskTarget.texture,
      this.trailInputNode,
      this.decayNode,
    );
  }

  get trailTexture(): THREE.Texture {
    return this.trailReadTarget.texture;
  }

  initializeFrame(): void {
    this.withRendererState(() => {
      this.render(this.previousTarget, this.copyMaterial);
      this.clear(this.rawMaskTarget);
      this.clear(this.blurTarget);
      this.clear(this.maskTarget);
      this.clear(this.trailReadTarget);
      this.clear(this.trailWriteTarget);
    });
  }

  analyzeFrame(config: Readonly<ResolvedCameraMotionConfig>): void {
    this.thresholdNode.value = config.threshold;
    this.radiusNode.value = config.blur;
    this.decayNode.value = config.decay;

    this.withRendererState(() => {
      this.render(this.rawMaskTarget, this.maskMaterial);
      this.render(this.blurTarget, this.horizontalBlurMaterial);
      this.render(this.maskTarget, this.verticalBlurMaterial);
      this.render(this.trailWriteTarget, this.trailMaterial);

      const previousReadTarget = this.trailReadTarget;
      this.trailReadTarget = this.trailWriteTarget;
      this.trailWriteTarget = previousReadTarget;
      this.trailInputNode.value = this.trailReadTarget.texture;

      this.render(this.previousTarget, this.copyMaterial);
    });
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.previousTarget.dispose();
    this.rawMaskTarget.dispose();
    this.blurTarget.dispose();
    this.maskTarget.dispose();
    this.trailReadTarget.dispose();
    this.trailWriteTarget.dispose();
    this.copyMaterial.dispose();
    this.maskMaterial.dispose();
    this.horizontalBlurMaterial.dispose();
    this.verticalBlurMaterial.dispose();
    this.trailMaterial.dispose();
  }

  private render(target: THREE.RenderTarget, material: NodeMaterial): void {
    this.renderer.setRenderTarget(target);
    this.quad.material = material;
    this.quad.render(this.renderer);
  }

  private clear(target: THREE.RenderTarget): void {
    this.renderer.setRenderTarget(target);
    this.renderer.clear();
  }

  private withRendererState(renderPasses: () => void): void {
    withRendererState(this.renderer, renderPasses);
  }
}

export class CameraMotionAnalyzer {
  readonly nodes: CameraMotionNodes;
  private config: ResolvedCameraMotionConfig = { ...DEFAULT_CAMERA_MOTION_CONFIG };
  private readonly createPipeline: CameraMotionPipelineFactory;
  private readonly onError: (error: unknown) => void;
  private sourceTexture: THREE.VideoTexture | null = null;
  private sourceWidth = 0;
  private sourceHeight = 0;
  private pipeline: CameraMotionPipeline | null = null;
  private lastFrameTime: number | null = null;
  private historyInitialized = false;
  private failedForCurrentSource = false;
  private disposed = false;
  private readonly fallbackTexture: THREE.DataTexture;

  constructor(
    renderer: Renderer,
    readonly data: CameraMotionData,
    options: CameraMotionAnalyzerOptions = {},
  ) {
    this.fallbackTexture = new THREE.DataTexture(
      new Uint8Array([0]),
      1,
      1,
      THREE.RedFormat,
    );
    this.fallbackTexture.name = "CameraMotion.fallback";
    this.fallbackTexture.colorSpace = THREE.NoColorSpace;
    this.fallbackTexture.magFilter = THREE.LinearFilter;
    this.fallbackTexture.minFilter = THREE.LinearFilter;
    this.fallbackTexture.generateMipmaps = false;
    this.fallbackTexture.needsUpdate = true;
    this.nodes = {
      maskNode: TSL.texture(this.fallbackTexture),
      trailNode: TSL.texture(this.fallbackTexture),
    };
    this.createPipeline = options.createPipeline
      ?? ((texture, width, height) => new GpuCameraMotionPipeline(renderer, texture, width, height));
    this.onError = options.onError ?? ((error) => console.error("Camera motion analysis failed:", error));
    this.setInactive();
  }

  getConfig(): Readonly<ResolvedCameraMotionConfig> {
    return this.config;
  }

  configure(config?: CameraMotionConfig): void {
    if (this.disposed) return;
    this.config = resolveCameraMotionConfig(config);
    this.lastFrameTime = null;
    this.historyInitialized = false;
    this.failedForCurrentSource = false;
    this.setInactive();
    if (!this.config.enabled) this.releaseSource();
  }

  update(camera: CameraMotionInput): void {
    if (this.disposed || !this.config.enabled || !camera.active || !camera.texture) {
      this.releaseSource();
      return;
    }

    if (
      this.sourceTexture !== camera.texture
      || this.sourceWidth !== camera.width
      || this.sourceHeight !== camera.height
    ) {
      this.releasePipeline();
      this.sourceTexture = camera.texture;
      this.sourceWidth = camera.width;
      this.sourceHeight = camera.height;
      this.failedForCurrentSource = false;
    }

    if (this.failedForCurrentSource) return;
    const frameTime = getVideoFrameTime(camera.texture);
    if (frameTime === null || frameTime === this.lastFrameTime) return;

    try {
      if (!this.pipeline) {
        const size = calculateMotionAnalysisSize(camera.width, camera.height);
        if (size.width === 0 || size.height === 0) return;
        this.pipeline = this.createPipeline(camera.texture, size.width, size.height);
      }

      if (!this.historyInitialized) {
        this.pipeline.initializeFrame();
        this.historyInitialized = true;
        this.lastFrameTime = frameTime;
        this.setInactive();
        return;
      }

      this.pipeline.analyzeFrame(this.config);
      this.lastFrameTime = frameTime;
      this.data.active = true;
      this.data.maskTexture = this.pipeline.maskTexture;
      this.data.trailTexture = this.pipeline.trailTexture;
      this.nodes.maskNode.value = this.pipeline.maskTexture;
      this.nodes.trailNode.value = this.pipeline.trailTexture;
      const size = calculateMotionAnalysisSize(camera.width, camera.height);
      this.data.width = size.width;
      this.data.height = size.height;
    } catch (error) {
      this.releasePipeline();
      this.failedForCurrentSource = true;
      this.onError(error);
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.releaseSource();
    this.fallbackTexture.dispose();
    this.disposed = true;
  }

  private releasePipeline(): void {
    this.pipeline?.dispose();
    this.pipeline = null;
    this.lastFrameTime = null;
    this.historyInitialized = false;
    this.setInactive();
  }

  private releaseSource(): void {
    this.releasePipeline();
    this.sourceTexture = null;
    this.sourceWidth = 0;
    this.sourceHeight = 0;
    this.failedForCurrentSource = false;
  }

  private setInactive(): void {
    setDataInactive(this.data);
    this.nodes.maskNode.value = this.fallbackTexture;
    this.nodes.trailNode.value = this.fallbackTexture;
  }
}
