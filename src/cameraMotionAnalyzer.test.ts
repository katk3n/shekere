import { describe, expect, it, vi } from "vitest";
import type * as THREE from "three";
import type { Renderer } from "three/webgpu";
import type { CameraMotionData } from "./cameraManager";
import {
  CameraMotionAnalyzer,
  calculateMotionAnalysisSize,
  resolveCameraMotionConfig,
  type CameraMotionPipeline,
} from "./cameraMotionAnalyzer";

function createMotionData(): CameraMotionData {
  return {
    active: false,
    maskTexture: null,
    trailTexture: null,
    width: 0,
    height: 0,
  };
}

describe("resolveCameraMotionConfig", () => {
  it("applies ADR 0008 defaults", () => {
    expect(resolveCameraMotionConfig()).toEqual({
      enabled: false,
      threshold: 0.08,
      blur: 6,
      decay: 0.94,
    });
  });

  it("clamps numeric settings to the public contract", () => {
    expect(resolveCameraMotionConfig({
      enabled: true,
      threshold: 2,
      blur: -4,
      decay: 1,
    })).toEqual({
      enabled: true,
      threshold: 1,
      blur: 0,
      decay: 0.999,
    });
  });

  it("uses defaults for non-finite values", () => {
    expect(resolveCameraMotionConfig({
      threshold: Number.NaN,
      blur: Number.POSITIVE_INFINITY,
      decay: Number.NEGATIVE_INFINITY,
    })).toMatchObject({ threshold: 0.08, blur: 6, decay: 0.94 });
  });
});

describe("calculateMotionAnalysisSize", () => {
  it.each([
    [1280, 720, { width: 320, height: 180 }],
    [720, 1280, { width: 180, height: 320 }],
    [1, 10_000, { width: 1, height: 320 }],
    [0, 720, { width: 0, height: 0 }],
  ])("scales %sx%s to the ADR analysis size", (width, height, expected) => {
    expect(calculateMotionAnalysisSize(width, height)).toEqual(expected);
  });
});

describe("CameraMotionAnalyzer lifecycle", () => {
  function createVideoTexture(currentTime = 1): THREE.VideoTexture {
    return {
      image: { currentTime },
    } as unknown as THREE.VideoTexture;
  }

  function createPipeline(): CameraMotionPipeline {
    return {
      maskTexture: { name: "mask" } as THREE.Texture,
      trailTexture: { name: "trail" } as THREE.Texture,
      initializeFrame: vi.fn(),
      analyzeFrame: vi.fn(),
      dispose: vi.fn(),
    };
  }

  const renderer = {} as Renderer;

  it("keeps public identity stable and suppresses the first frame", () => {
    const data = createMotionData();
    const pipeline = createPipeline();
    const createPipelineFactory = vi.fn(() => pipeline);
    const analyzer = new CameraMotionAnalyzer(renderer, data, {
      createPipeline: createPipelineFactory,
    });
    const nodes = analyzer.nodes;
    const maskNode = nodes.maskNode;
    const trailNode = nodes.trailNode;
    const fallbackTexture = trailNode.value;
    const texture = createVideoTexture();

    analyzer.configure({ enabled: true });
    analyzer.update({ active: true, texture, width: 1280, height: 720 });

    expect(analyzer.data).toBe(data);
    expect(analyzer.nodes).toBe(nodes);
    expect(nodes.maskNode).toBe(maskNode);
    expect(nodes.trailNode).toBe(trailNode);
    expect(nodes.maskNode.value).toBe(fallbackTexture);
    expect(nodes.trailNode.value).toBe(fallbackTexture);
    expect(data).toEqual(createMotionData());
    expect(createPipelineFactory).toHaveBeenCalledWith(texture, 320, 180);
    expect(pipeline.initializeFrame).toHaveBeenCalledOnce();
    expect(pipeline.analyzeFrame).not.toHaveBeenCalled();
  });

  it("returns to the inactive contract on disable and dispose", () => {
    const data = createMotionData();
    const pipeline = createPipeline();
    const analyzer = new CameraMotionAnalyzer(renderer, data, {
      createPipeline: () => pipeline,
    });
    const fallbackTexture = analyzer.nodes.trailNode.value;
    const fallbackDispose = vi.spyOn(fallbackTexture, "dispose");
    const texture = createVideoTexture();

    analyzer.configure({ enabled: true });
    analyzer.update({ active: true, texture, width: 1280, height: 720 });

    data.active = true;
    data.maskTexture = {} as THREE.Texture;
    data.trailTexture = {} as THREE.Texture;
    data.width = 320;
    data.height = 180;
    analyzer.configure({ enabled: false });
    expect(data).toEqual(createMotionData());
    expect(pipeline.dispose).toHaveBeenCalledOnce();
    expect(analyzer.nodes.maskNode.value).toBe(fallbackTexture);
    expect(analyzer.nodes.trailNode.value).toBe(fallbackTexture);

    data.active = true;
    analyzer.dispose();
    expect(data).toEqual(createMotionData());
    expect(fallbackDispose).toHaveBeenCalledOnce();
  });

  it("analyzes no more than once for each new camera frame", () => {
    const data = createMotionData();
    const pipeline = createPipeline();
    const analyzer = new CameraMotionAnalyzer(renderer, data, {
      createPipeline: () => pipeline,
    });
    const texture = createVideoTexture(1);

    analyzer.configure({ enabled: true, threshold: 0.2, blur: 4, decay: 0.8 });
    analyzer.update({ active: true, texture, width: 1280, height: 720 });
    analyzer.update({ active: true, texture, width: 1280, height: 720 });
    expect(pipeline.analyzeFrame).not.toHaveBeenCalled();

    (texture.image as { currentTime: number }).currentTime = 2;
    analyzer.update({ active: true, texture, width: 1280, height: 720 });
    analyzer.update({ active: true, texture, width: 1280, height: 720 });

    expect(pipeline.analyzeFrame).toHaveBeenCalledOnce();
    expect(pipeline.analyzeFrame).toHaveBeenCalledWith({
      enabled: true,
      threshold: 0.2,
      blur: 4,
      decay: 0.8,
    });
    expect(data).toEqual({
      active: true,
      maskTexture: pipeline.maskTexture,
      trailTexture: pipeline.trailTexture,
      width: 320,
      height: 180,
    });
    expect(analyzer.nodes.maskNode.value).toBe(pipeline.maskTexture);
    expect(analyzer.nodes.trailNode.value).toBe(pipeline.trailTexture);
  });

  it("reuses compatible targets but resets history on sketch reload", () => {
    const data = createMotionData();
    const pipeline = createPipeline();
    const createPipelineFactory = vi.fn(() => pipeline);
    const analyzer = new CameraMotionAnalyzer(renderer, data, {
      createPipeline: createPipelineFactory,
    });
    const texture = createVideoTexture(1);
    const camera = { active: true, texture, width: 1280, height: 720 };

    analyzer.configure({ enabled: true });
    analyzer.update(camera);
    analyzer.configure({ enabled: true });
    analyzer.update(camera);

    expect(createPipelineFactory).toHaveBeenCalledOnce();
    expect(pipeline.initializeFrame).toHaveBeenCalledTimes(2);
    expect(data.active).toBe(false);
    expect(analyzer.nodes.maskNode.value.name).toBe("CameraMotion.fallback");
    expect(analyzer.nodes.trailNode.value.name).toBe("CameraMotion.fallback");
  });

  it("disposes resources when the camera texture or dimensions change", () => {
    const data = createMotionData();
    const pipelines = [createPipeline(), createPipeline(), createPipeline()];
    const createPipelineFactory = vi.fn()
      .mockReturnValueOnce(pipelines[0])
      .mockReturnValueOnce(pipelines[1])
      .mockReturnValueOnce(pipelines[2]);
    const analyzer = new CameraMotionAnalyzer(renderer, data, {
      createPipeline: createPipelineFactory,
    });
    const firstTexture = createVideoTexture(1);
    const secondTexture = createVideoTexture(1);

    analyzer.configure({ enabled: true });
    analyzer.update({ active: true, texture: firstTexture, width: 1280, height: 720 });
    analyzer.update({ active: true, texture: secondTexture, width: 1280, height: 720 });
    expect(pipelines[0].dispose).toHaveBeenCalledOnce();

    (secondTexture.image as { currentTime: number }).currentTime = 2;
    analyzer.update({ active: true, texture: secondTexture, width: 640, height: 480 });
    expect(pipelines[1].dispose).toHaveBeenCalledOnce();
    expect(createPipelineFactory).toHaveBeenLastCalledWith(secondTexture, 320, 240);
  });

  it("fails safely without retrying every render frame", () => {
    const data = createMotionData();
    const failure = new Error("GPU failed");
    const createPipelineFactory = vi.fn(() => { throw failure; });
    const onError = vi.fn();
    const analyzer = new CameraMotionAnalyzer(renderer, data, {
      createPipeline: createPipelineFactory,
      onError,
    });
    const texture = createVideoTexture(1);
    const camera = { active: true, texture, width: 1280, height: 720 };

    analyzer.configure({ enabled: true });
    analyzer.update(camera);
    (texture.image as { currentTime: number }).currentTime = 2;
    analyzer.update(camera);

    expect(createPipelineFactory).toHaveBeenCalledOnce();
    expect(onError).toHaveBeenCalledWith(failure);
    expect(data).toEqual(createMotionData());
  });
});
