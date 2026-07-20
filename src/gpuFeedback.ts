import * as THREE from "three";
import {
  NodeMaterial,
  QuadMesh,
  type Node,
  type Renderer,
  type TextureNode,
  type UniformNode,
} from "three/webgpu";
import * as TSL from "three/tsl";
import { withRendererState } from "./rendererState";

export type FeedbackUniformValue =
  | number
  | [number, number]
  | [number, number, number]
  | [number, number, number, number];

export interface FeedbackBuildContext {
  previous: TextureNode;
  textures: Record<string, TextureNode>;
  uniforms: Record<string, UniformNode<unknown, UniformRuntimeValue>>;
  uv: Node;
  deltaTime: UniformNode<unknown, number>;
  time: UniformNode<unknown, number>;
}

export interface FeedbackPassOptions {
  name?: string;
  width: number;
  height: number;
  format?: "rgba8" | "rgba16f";
  clearValue?: [number, number, number, number];
  textures?: string[];
  uniforms?: Record<string, FeedbackUniformValue>;
  build: (context: FeedbackBuildContext) => Node;
}

export type FeedbackTextureInput = THREE.Texture | TextureNode | FeedbackPass | null;

export interface FeedbackPassUpdate {
  textures?: Record<string, FeedbackTextureInput>;
  uniforms?: Record<string, FeedbackUniformValue>;
}

export interface FeedbackPass {
  readonly node: TextureNode;
  readonly texture: THREE.Texture | null;
  readonly width: number;
  readonly height: number;
  update(values?: FeedbackPassUpdate): void;
  clear(): void;
  dispose(): void;
}

export interface ShekereGpuApi {
  createFeedbackPass(options: FeedbackPassOptions): FeedbackPass;
}

type UniformRuntimeValue = number | THREE.Vector2 | THREE.Vector3 | THREE.Vector4;

interface TextureBinding {
  dependency: FeedbackPassImpl | null;
  resolve: () => THREE.Texture | null;
}

interface FeedbackScope {
  readonly passes: Set<FeedbackPassImpl>;
  logicalPixels: number;
  disposed: boolean;
}

interface FeedbackServiceOptions {
  onError?: (error: Error) => void;
  supportsRgba16f?: () => boolean;
  runWithRendererState?: <T>(renderer: Renderer, operation: () => T) => T;
}

const MAX_DIMENSION = 1024;
const MAX_PASSES = 8;
const MAX_LOGICAL_PIXELS = 2_097_152;

function createBlackFallback(): THREE.DataTexture {
  const texture = new THREE.DataTexture(new Uint8Array([0, 0, 0, 0]), 1, 1, THREE.RGBAFormat);
  texture.name = "Shekere.feedback.fallback";
  texture.colorSpace = THREE.NoColorSpace;
  texture.generateMipmaps = false;
  texture.needsUpdate = true;
  return texture;
}

function createScope(): FeedbackScope {
  return { passes: new Set(), logicalPixels: 0, disposed: false };
}

function isFiniteTuple(value: unknown, size: number): value is number[] {
  return Array.isArray(value)
    && value.length === size
    && value.every((component) => typeof component === "number" && Number.isFinite(component));
}

function uniformDimension(value: FeedbackUniformValue): number {
  return typeof value === "number" ? 1 : value.length;
}

function validateUniformValue(value: unknown, expectedDimension?: number): asserts value is FeedbackUniformValue {
  const dimension = typeof value === "number"
    ? (Number.isFinite(value) ? 1 : 0)
    : Array.isArray(value) && value.length >= 2 && value.length <= 4
      && value.every((component) => typeof component === "number" && Number.isFinite(component))
      ? value.length
      : 0;
  if (dimension === 0 || (expectedDimension !== undefined && dimension !== expectedDimension)) {
    throw new Error(expectedDimension === undefined
      ? "Feedback uniform values must be finite scalars or vectors of 2–4 components."
      : `Feedback uniform must keep its original ${expectedDimension}-component dimension.`);
  }
}

function toUniformRuntimeValue(value: FeedbackUniformValue): UniformRuntimeValue {
  if (typeof value === "number") return value;
  if (value.length === 2) return new THREE.Vector2(...value);
  if (value.length === 3) return new THREE.Vector3(...value);
  return new THREE.Vector4(...value);
}

function createUniformNode(value: FeedbackUniformValue): UniformNode<unknown, UniformRuntimeValue> {
  const runtimeValue = toUniformRuntimeValue(value);
  if (typeof runtimeValue === "number") {
    return TSL.uniform(runtimeValue) as UniformNode<unknown, UniformRuntimeValue>;
  }
  if (runtimeValue instanceof THREE.Vector2) {
    return TSL.uniform(runtimeValue) as UniformNode<unknown, UniformRuntimeValue>;
  }
  if (runtimeValue instanceof THREE.Vector3) {
    return TSL.uniform(runtimeValue) as UniformNode<unknown, UniformRuntimeValue>;
  }
  return TSL.uniform(runtimeValue) as UniformNode<unknown, UniformRuntimeValue>;
}

function assignUniformValue(target: UniformRuntimeValue, value: FeedbackUniformValue): UniformRuntimeValue {
  if (typeof value === "number") return value;
  if (target instanceof THREE.Vector2 && value.length === 2) return target.set(...value);
  if (target instanceof THREE.Vector3 && value.length === 3) return target.set(...value);
  if (target instanceof THREE.Vector4 && value.length === 4) return target.set(...value);
  return toUniformRuntimeValue(value);
}

function createRenderTarget(width: number, height: number, format: "rgba8" | "rgba16f", name: string): THREE.RenderTarget {
  const target = new THREE.RenderTarget(width, height, {
    depthBuffer: false,
    format: THREE.RGBAFormat,
    type: format === "rgba16f" ? THREE.HalfFloatType : THREE.UnsignedByteType,
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

export class GpuFeedbackService implements ShekereGpuApi {
  private readonly fallbackTexture = createBlackFallback();
  private readonly onError: (error: Error) => void;
  private readonly supportsRgba16f: () => boolean;
  private readonly runWithRendererState: <T>(renderer: Renderer, operation: () => T) => T;
  private readonly publicNodes = new WeakMap<object, FeedbackPassImpl>();
  private activeScope: FeedbackScope | null = null;
  private candidateScope: FeedbackScope | null = null;
  private nextCreationIndex = 0;
  private lastFrameTime: number | null = null;
  private disposed = false;

  constructor(private readonly renderer: Renderer, options: FeedbackServiceOptions = {}) {
    this.onError = options.onError ?? ((error) => console.error("Sketch GPU feedback error:", error));
    this.runWithRendererState = options.runWithRendererState ?? withRendererState;
    this.supportsRgba16f = options.supportsRgba16f ?? (() => {
      const backend = (this.renderer as unknown as {
        backend?: { isWebGPUBackend?: boolean; extensions?: { has?: (name: string) => boolean } };
      }).backend;
      return backend?.isWebGPUBackend === true || backend?.extensions?.has?.("EXT_color_buffer_float") === true;
    });
  }

  createFeedbackPass(options: FeedbackPassOptions): FeedbackPass {
    if (this.disposed) throw new Error("The Shekere GPU service has been disposed.");
    const scope = this.candidateScope ?? this.activeScope;
    if (!scope || scope.disposed) {
      throw new Error("Feedback passes can only be created inside an active sketch scope.");
    }
    this.validateCreation(options, scope);

    const pass = new FeedbackPassImpl(
      this,
      scope,
      this.renderer,
      this.fallbackTexture,
      this.runWithRendererState,
      this.nextCreationIndex++,
      options,
    );
    scope.passes.add(pass);
    scope.logicalPixels += pass.width * pass.height;
    this.publicNodes.set(pass.node, pass);
    return pass;
  }

  beginCandidateScope(): void {
    if (this.disposed) throw new Error("The Shekere GPU service has been disposed.");
    if (this.candidateScope) this.disposeScope(this.candidateScope);
    this.candidateScope = createScope();
  }

  commitCandidateScope(): void {
    if (!this.candidateScope) throw new Error("No candidate sketch GPU scope is open.");
    if (this.activeScope) this.disposeScope(this.activeScope);
    this.activeScope = this.candidateScope;
    this.candidateScope = null;
  }

  rollbackCandidateScope(): void {
    if (!this.candidateScope) return;
    this.disposeScope(this.candidateScope);
    this.candidateScope = null;
  }

  disposeActiveScope(): void {
    if (!this.activeScope) return;
    this.disposeScope(this.activeScope);
    this.activeScope = null;
  }

  executeQueued(time: number): void {
    if (this.disposed || !this.activeScope) return;
    const safeTime = Math.max(
      this.lastFrameTime ?? 0,
      Number.isFinite(time) ? Math.max(0, time) : 0,
    );
    const deltaTime = this.lastFrameTime === null ? 0 : Math.min(0.1, Math.max(0, safeTime - this.lastFrameTime));
    this.lastFrameTime = safeTime;
    for (const pass of [...this.activeScope.passes]) {
      if (pass.isQueued) pass.execute(safeTime, deltaTime);
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.rollbackCandidateScope();
    this.disposeActiveScope();
    this.fallbackTexture.dispose();
  }

  resolveTextureInput(input: FeedbackTextureInput, consumer: FeedbackPassImpl): TextureBinding {
    if (input === null) return { dependency: null, resolve: () => null };
    if (input instanceof THREE.Texture) return { dependency: null, resolve: () => input };

    if (input instanceof FeedbackPassImpl) {
      if (input.service !== this || input.scope !== consumer.scope) {
        throw new Error("A feedback dependency must belong to the same sketch scope.");
      }
      if (input.creationIndex >= consumer.creationIndex) {
        throw new Error("Feedback dependencies must reference an earlier-created pass.");
      }
      return { dependency: input, resolve: () => input.texture };
    }

    if (typeof input === "object" && this.publicNodes.has(input)) {
      throw new Error("Pass FeedbackPass itself as a dependency, not FeedbackPass.node.");
    }

    if (typeof input === "object" && input !== null && "isTextureNode" in input && "value" in input) {
      const node = input as TextureNode;
      return {
        dependency: null,
        resolve: () => node.value instanceof THREE.Texture ? node.value : null,
      };
    }
    throw new Error("Feedback texture inputs must be a Texture, TextureNode, FeedbackPass, or null.");
  }

  report(error: unknown, passName: string): void {
    const detail = error instanceof Error ? error.message : String(error);
    this.onError(new Error(`[${passName}] ${detail}`));
  }

  unregister(pass: FeedbackPassImpl): void {
    if (!pass.scope.passes.delete(pass)) return;
    pass.scope.logicalPixels -= pass.width * pass.height;
  }

  private validateCreation(options: FeedbackPassOptions, scope: FeedbackScope): void {
    if (!Number.isInteger(options.width) || options.width < 1 || options.width > MAX_DIMENSION
      || !Number.isInteger(options.height) || options.height < 1 || options.height > MAX_DIMENSION) {
      throw new Error(`Feedback width and height must be integers from 1 through ${MAX_DIMENSION}.`);
    }
    if (scope.passes.size >= MAX_PASSES) throw new Error(`A sketch may own at most ${MAX_PASSES} feedback passes.`);
    if (scope.logicalPixels + options.width * options.height > MAX_LOGICAL_PIXELS) {
      throw new Error(`A sketch may own at most ${MAX_LOGICAL_PIXELS} logical feedback pixels.`);
    }
    if (options.format !== undefined && options.format !== "rgba8" && options.format !== "rgba16f") {
      throw new Error("Feedback format must be rgba8 or rgba16f.");
    }
    if (options.format === "rgba16f" && !this.supportsRgba16f()) {
      throw new Error("rgba16f feedback is not supported by the active renderer backend.");
    }
    if (options.clearValue !== undefined && !isFiniteTuple(options.clearValue, 4)) {
      throw new Error("Feedback clearValue must contain four finite numbers.");
    }
    const textureNames = options.textures ?? [];
    if (new Set(textureNames).size !== textureNames.length || textureNames.some((name) => !name)) {
      throw new Error("Feedback texture names must be non-empty and unique.");
    }
    for (const value of Object.values(options.uniforms ?? {})) validateUniformValue(value);
    if (typeof options.build !== "function") throw new Error("Feedback build must be a function.");
  }

  private disposeScope(scope: FeedbackScope): void {
    if (scope.disposed) return;
    scope.disposed = true;
    for (const pass of [...scope.passes]) pass.dispose();
  }
}

class FeedbackPassImpl implements FeedbackPass {
  readonly node: TextureNode;
  readonly width: number;
  readonly height: number;
  readonly name: string;
  readonly material: NodeMaterial;
  readonly quad: QuadMesh;
  readonly previousNode: TextureNode;
  readonly textureNodes: Record<string, TextureNode> = {};
  readonly uniformNodes: Record<string, UniformNode<unknown, UniformRuntimeValue>> = {};
  readonly uniformDimensions = new Map<string, number>();
  readonly textureBindings = new Map<string, TextureBinding>();
  readonly timeNode = TSL.uniform(0);
  readonly deltaTimeNode = TSL.uniform(0);
  private readTarget: THREE.RenderTarget;
  private writeTarget: THREE.RenderTarget;
  private readonly clearValue: [number, number, number, number];
  private updateQueued = false;
  private clearQueued = false;
  private initialClearPending = true;
  private disposed = false;

  constructor(
    readonly service: GpuFeedbackService,
    readonly scope: FeedbackScope,
    private readonly renderer: Renderer,
    private readonly fallbackTexture: THREE.Texture,
    private readonly runWithRendererState: <T>(renderer: Renderer, operation: () => T) => T,
    readonly creationIndex: number,
    options: FeedbackPassOptions,
  ) {
    this.width = options.width;
    this.height = options.height;
    this.name = options.name?.trim() || `feedback-${creationIndex + 1}`;
    this.clearValue = options.clearValue ?? [0, 0, 0, 0];
    this.previousNode = TSL.texture(fallbackTexture);
    this.node = TSL.texture(fallbackTexture);

    for (const name of options.textures ?? []) {
      this.textureNodes[name] = TSL.texture(fallbackTexture);
      this.textureBindings.set(name, { dependency: null, resolve: () => null });
    }
    for (const [name, value] of Object.entries(options.uniforms ?? {})) {
      validateUniformValue(value);
      this.uniformNodes[name] = createUniformNode(value);
      this.uniformDimensions.set(name, uniformDimension(value));
    }

    const fragmentNode = options.build({
      previous: this.previousNode,
      textures: this.textureNodes,
      uniforms: this.uniformNodes,
      uv: TSL.uv(),
      deltaTime: this.deltaTimeNode,
      time: this.timeNode,
    });
    if (!fragmentNode || typeof fragmentNode !== "object" || !("isNode" in fragmentNode)) {
      throw new Error("Feedback build must return a TSL node.");
    }

    this.material = new NodeMaterial();
    this.material.name = `Shekere.feedback.${this.name}`;
    this.material.fragmentNode = fragmentNode;
    this.quad = new QuadMesh(this.material);
    const format = options.format ?? "rgba8";
    this.readTarget = createRenderTarget(this.width, this.height, format, `${this.material.name}.A`);
    this.writeTarget = createRenderTarget(this.width, this.height, format, `${this.material.name}.B`);
    this.previousNode.value = this.readTarget.texture;
    this.node.value = this.readTarget.texture;
  }

  get texture(): THREE.Texture | null {
    return this.disposed ? null : this.readTarget.texture;
  }

  get isQueued(): boolean {
    return !this.disposed && (this.updateQueued || this.clearQueued || this.initialClearPending);
  }

  update(values: FeedbackPassUpdate = {}): void {
    if (this.disposed) {
      this.service.report("Cannot update a disposed feedback pass.", this.name);
      return;
    }
    try {
      const bindings = new Map<string, TextureBinding>();
      for (const name of this.textureBindings.keys()) {
        bindings.set(name, { dependency: null, resolve: () => null });
      }
      for (const [name, input] of Object.entries(values.textures ?? {})) {
        if (!this.textureBindings.has(name)) throw new Error(`Unknown feedback texture: ${name}`);
        bindings.set(name, this.service.resolveTextureInput(input, this));
      }
      const uniforms = new Map<string, FeedbackUniformValue>();
      for (const [name, value] of Object.entries(values.uniforms ?? {})) {
        const dimension = this.uniformDimensions.get(name);
        if (dimension === undefined) throw new Error(`Unknown feedback uniform: ${name}`);
        validateUniformValue(value, dimension);
        uniforms.set(name, value);
      }

      for (const [name, binding] of bindings) this.textureBindings.set(name, binding);
      for (const [name, value] of uniforms) {
        const node = this.uniformNodes[name];
        node.value = assignUniformValue(node.value, value);
      }
      this.updateQueued = true;
    } catch (error) {
      this.service.report(error, this.name);
    }
  }

  clear(): void {
    if (this.disposed) {
      this.service.report("Cannot clear a disposed feedback pass.", this.name);
      return;
    }
    this.clearQueued = true;
  }

  execute(time: number, deltaTime: number): void {
    if (!this.isQueued) return;
    const shouldUpdate = this.updateQueued;
    const shouldClear = this.clearQueued || this.initialClearPending;
    this.updateQueued = false;
    this.clearQueued = false;
    this.initialClearPending = false;
    try {
      this.runWithRendererState(this.renderer, () => {
        if (shouldClear) {
          this.clearTarget(this.readTarget);
          this.clearTarget(this.writeTarget);
          this.previousNode.value = this.readTarget.texture;
          this.node.value = this.readTarget.texture;
        }
        if (!shouldUpdate) return;
        for (const [name, binding] of this.textureBindings) {
          this.textureNodes[name].value = binding.resolve() ?? this.fallbackTexture;
        }
        this.timeNode.value = time;
        this.deltaTimeNode.value = shouldClear ? 0 : deltaTime;
        this.renderer.setRenderTarget(this.writeTarget);
        this.quad.render(this.renderer);
        const previousReadTarget = this.readTarget;
        this.readTarget = this.writeTarget;
        this.writeTarget = previousReadTarget;
        this.previousNode.value = this.readTarget.texture;
        this.node.value = this.readTarget.texture;
      });
    } catch (error) {
      this.service.report(error, this.name);
      this.dispose();
    }
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.updateQueued = false;
    this.clearQueued = false;
    this.readTarget.dispose();
    this.writeTarget.dispose();
    this.material.dispose();
    this.previousNode.value = this.fallbackTexture;
    this.node.value = this.fallbackTexture;
    for (const textureNode of Object.values(this.textureNodes)) textureNode.value = this.fallbackTexture;
    this.textureBindings.clear();
    this.service.unregister(this);
  }

  private clearTarget(target: THREE.RenderTarget): void {
    this.renderer.setRenderTarget(target);
    this.renderer.setClearColor(
      new THREE.Color(this.clearValue[0], this.clearValue[1], this.clearValue[2]),
      this.clearValue[3],
    );
    this.renderer.clear();
  }
}
