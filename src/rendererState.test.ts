import { describe, expect, it, vi } from "vitest";
import { RendererUtils, type Renderer } from "three/webgpu";
import { withRendererState } from "./rendererState";

describe("withRendererState", () => {
  it("restores renderer state after successful offscreen work", () => {
    const renderer = {} as Renderer;
    const state = {} as ReturnType<typeof RendererUtils.saveRendererState>;
    const boundary = {
      saveRendererState: vi.fn(() => state),
      resetRendererState: vi.fn(() => state),
      restoreRendererState: vi.fn(),
    };

    expect(withRendererState(renderer, () => 42, boundary)).toBe(42);
    expect(boundary.restoreRendererState).toHaveBeenCalledWith(renderer, state);
  });

  it("restores renderer state when offscreen work throws", () => {
    const renderer = {} as Renderer;
    const state = {} as ReturnType<typeof RendererUtils.saveRendererState>;
    const boundary = {
      saveRendererState: vi.fn(() => state),
      resetRendererState: vi.fn(() => state),
      restoreRendererState: vi.fn(),
    };
    const failure = new Error("render failed");

    expect(() => withRendererState(renderer, () => { throw failure; }, boundary)).toThrow(failure);
    expect(boundary.restoreRendererState).toHaveBeenCalledWith(renderer, state);
  });
});
