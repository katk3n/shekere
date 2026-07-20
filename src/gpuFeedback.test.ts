import * as THREE from "three";
import { describe, expect, it, vi } from "vitest";
import type { Renderer } from "three/webgpu";
import * as TSL from "three/tsl";
import {
  GpuFeedbackService,
  type FeedbackBuildContext,
  type FeedbackPassOptions,
} from "./gpuFeedback";

function createHarness(render = vi.fn()) {
  const renderer = {
    setRenderTarget: vi.fn(),
    setClearColor: vi.fn(),
    clear: vi.fn(),
    render,
  } as unknown as Renderer;
  const errors: Error[] = [];
  const service = new GpuFeedbackService(renderer, {
    onError: (error) => errors.push(error),
    supportsRgba16f: () => true,
    runWithRendererState: (_renderer, operation) => operation(),
  });
  service.beginCandidateScope();
  return { renderer, service, errors, render };
}

function passOptions(overrides: Partial<FeedbackPassOptions> = {}): FeedbackPassOptions {
  return {
    width: 8,
    height: 4,
    build: ({ previous }) => previous.sample(TSL.uv()),
    ...overrides,
  };
}

describe("GpuFeedbackService", () => {
  it("creates two-state ping-pong output with a stable public node", () => {
    const { service, render } = createHarness();
    const pass = service.createFeedbackPass(passOptions());
    const node = pass.node;
    const firstTexture = pass.texture;
    service.commitCandidateScope();

    pass.update();
    service.executeQueued(1);

    expect(render).toHaveBeenCalledOnce();
    expect(pass.node).toBe(node);
    expect(pass.texture).not.toBe(firstTexture);
    expect(pass.node.value).toBe(pass.texture);
  });

  it("does no work when a pass is not queued", () => {
    const { service, render } = createHarness();
    const pass = service.createFeedbackPass(passOptions());
    const texture = pass.texture;
    service.commitCandidateScope();

    service.executeQueued(1);

    expect(render).not.toHaveBeenCalled();
    expect(pass.texture).toBe(texture);
  });

  it("coalesces updates and preserves all inputs after an invalid atomic update", () => {
    const { service, errors } = createHarness();
    let context: FeedbackBuildContext | undefined;
    const pass = service.createFeedbackPass(passOptions({
      textures: ["source"],
      uniforms: { gain: 1 },
      build: (value) => {
        context = value;
        return value.previous.sample(value.uv);
      },
    }));
    service.commitCandidateScope();
    const source = new THREE.Texture();

    pass.update({ textures: { source }, uniforms: { gain: 2 } });
    pass.update({ textures: { source }, uniforms: { gain: 3 } });
    pass.update({ textures: { source: null }, uniforms: { unknown: 4 } });
    service.executeQueued(1);

    expect(context?.uniforms.gain.value).toBe(3);
    expect(context?.textures.source.value).toBe(source);
    expect(errors).toHaveLength(1);
  });

  it("accepts only earlier pass dependencies and rejects public pass nodes", () => {
    const { service, errors } = createHarness();
    const producer = service.createFeedbackPass(passOptions({ textures: ["state"] }));
    const consumer = service.createFeedbackPass(passOptions({ textures: ["state"] }));
    service.commitCandidateScope();

    consumer.update({ textures: { state: producer } });
    producer.update({ textures: { state: consumer } });
    consumer.update({ textures: { state: consumer.node } });

    const messages = errors.map((error) => error.message).join(" ");
    expect(messages).toContain("earlier-created pass");
    expect(messages).toContain("FeedbackPass itself");
  });

  it("resolves a stable host texture node immediately before execution", () => {
    const { service } = createHarness();
    let context: FeedbackBuildContext | undefined;
    const pass = service.createFeedbackPass(passOptions({
      textures: ["camera"],
      build: (value) => {
        context = value;
        return value.previous.sample(value.uv);
      },
    }));
    service.commitCandidateScope();
    const first = new THREE.Texture();
    const latest = new THREE.Texture();
    const stableNode = TSL.texture(first);
    pass.update({ textures: { camera: stableNode } });
    stableNode.value = latest;

    service.executeQueued(1);

    expect(context?.textures.camera.value).toBe(latest);
  });

  it("enforces pass, pixel, size, clear-value, and format limits before creation", () => {
    const { service } = createHarness();
    for (let index = 0; index < 8; index += 1) {
      service.createFeedbackPass(passOptions({ width: 1, height: 1 }));
    }
    expect(() => service.createFeedbackPass(passOptions({ width: 1, height: 1 }))).toThrow("at most 8");

    const second = createHarness().service;
    second.createFeedbackPass(passOptions({ width: 1024, height: 1024 }));
    second.createFeedbackPass(passOptions({ width: 1024, height: 1024 }));
    expect(() => second.createFeedbackPass(passOptions({ width: 1, height: 1 }))).toThrow("logical feedback pixels");
    expect(() => createHarness().service.createFeedbackPass(passOptions({ width: 0 }))).toThrow("integers");
    expect(() => createHarness().service.createFeedbackPass(passOptions({
      clearValue: [0, 0, Number.NaN, 1],
    }))).toThrow("clearValue");

    const unsupported = new GpuFeedbackService({} as Renderer, { supportsRgba16f: () => false });
    unsupported.beginCandidateScope();
    expect(() => unsupported.createFeedbackPass(passOptions({ format: "rgba16f" }))).toThrow("not supported");
  });

  it("clears both targets without rendering, then disposes idempotently", () => {
    const { service, renderer, render, errors } = createHarness();
    const pass = service.createFeedbackPass(passOptions());
    const node = pass.node;
    service.commitCandidateScope();

    pass.clear();
    service.executeQueued(1);
    expect(renderer.clear).toHaveBeenCalledTimes(2);
    expect(render).not.toHaveBeenCalled();

    pass.dispose();
    pass.dispose();
    expect(pass.texture).toBeNull();
    expect(pass.node).toBe(node);
    expect(pass.node.value.name).toBe("Shekere.feedback.fallback");
    pass.update();
    pass.clear();
    expect(errors).toHaveLength(2);
  });

  it("rolls candidate scopes back and isolates a failing pass", () => {
    let calls = 0;
    const render = vi.fn(() => {
      calls += 1;
      if (calls === 1) throw new Error("compile failed");
    });
    const { service, errors } = createHarness(render);
    const failing = service.createFeedbackPass(passOptions({ name: "failing" }));
    const healthy = service.createFeedbackPass(passOptions({ name: "healthy" }));
    service.commitCandidateScope();
    failing.update();
    healthy.update();

    service.executeQueued(1);

    expect(failing.texture).toBeNull();
    expect(healthy.texture).not.toBeNull();
    expect(render).toHaveBeenCalledTimes(2);
    expect(errors[0].message).toContain("failing");

    service.beginCandidateScope();
    const candidate = service.createFeedbackPass(passOptions());
    service.rollbackCandidateScope();
    expect(candidate.texture).toBeNull();
  });

  it("commits replacement scopes and disposes every active resource on shutdown", () => {
    const { service } = createHarness();
    const original = service.createFeedbackPass(passOptions());
    service.commitCandidateScope();
    service.beginCandidateScope();
    const replacement = service.createFeedbackPass(passOptions());

    service.commitCandidateScope();
    expect(original.texture).toBeNull();
    expect(replacement.texture).not.toBeNull();

    service.dispose();
    service.dispose();
    expect(replacement.texture).toBeNull();
  });

  it("uses monotonic frame time and caps delta time at 0.1 seconds", () => {
    const { service } = createHarness();
    let context: FeedbackBuildContext | undefined;
    const pass = service.createFeedbackPass(passOptions({
      build: (value) => {
        context = value;
        return value.previous.sample(value.uv);
      },
    }));
    service.commitCandidateScope();

    pass.update();
    service.executeQueued(1);
    pass.update();
    service.executeQueued(2);
    expect(context?.time.value).toBe(2);
    expect(context?.deltaTime.value).toBe(0.1);

    pass.update();
    service.executeQueued(1.5);

    expect(context?.time.value).toBe(2);
    expect(context?.deltaTime.value).toBe(0);
  });
});
