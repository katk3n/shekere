import { RendererUtils, type Renderer } from "three/webgpu";

interface RendererStateBoundary {
  saveRendererState: typeof RendererUtils.saveRendererState;
  resetRendererState: typeof RendererUtils.resetRendererState;
  restoreRendererState: typeof RendererUtils.restoreRendererState;
}

/** Run host-owned offscreen work without leaking renderer state to the caller. */
export function withRendererState<T>(
  renderer: Renderer,
  operation: () => T,
  boundary: RendererStateBoundary = RendererUtils,
): T {
  const state = boundary.saveRendererState(renderer);
  boundary.resetRendererState(renderer, state);
  try {
    return operation();
  } finally {
    boundary.restoreRendererState(renderer, state);
  }
}
