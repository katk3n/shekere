# GPU Feedback

`Shekere.gpu` lets a sketch keep evolving image or simulation state on the GPU
without receiving the renderer or raw render targets. It is suitable for
ripples, accumulation, smoke-like effects, and texture-state particles.

## Creating and advancing a pass

Create a pass during module evaluation, `setup`, or a later `update`. The
`build` callback runs exactly once and must return the TSL node that calculates
the next state:

```javascript
this.feedback = Shekere.gpu.createFeedbackPass({
  name: "fade",
  width: 320,
  height: 180,
  format: "rgba16f",
  textures: ["seed"],
  uniforms: { decay: 0.97 },
  clearValue: [0, 0, 0, 0],
  build({ previous, textures, uniforms, uv, time, deltaTime }) {
    const oldValue = previous.sample(uv);
    const seed = textures.seed.sample(uv);
    return TSL.max(oldValue.mul(uniforms.decay), seed);
  }
});

export function update() {
  this.feedback.update({
    textures: { seed: Shekere.camera.motion.maskNode },
    uniforms: { decay: 0.98 }
  });
}
```

`update()` queues at most one execution for the current sketch frame. Multiple
calls coalesce and the last valid values win. A pass that is not queued keeps
its state without consuming an offscreen render pass. `deltaTime` is capped at
0.1 seconds after a stall.

## Inputs and dependencies

Texture inputs accept `THREE.Texture`, a TSL texture node, an earlier-created
`FeedbackPass`, or `null`. Missing and null inputs sample Shekere's black
fallback. When using another pass, pass the `FeedbackPass` object—not its
`node`—so Shekere can validate creation order. Arbitrary graph reordering and
forward dependencies are not supported.

Uniforms are finite scalars or arrays of two, three, or four numbers. Names and
dimensions are fixed at creation. An update containing an unknown name, wrong
dimension, or non-finite number is rejected atomically; the previous valid
inputs remain active.

## Sampling and ownership

`pass.node` is the preferred TSL output. Its identity stays stable while the
host swaps ping-pong textures. `pass.texture` is the current raw output and may
change after every execution. It becomes `null` after disposal or pass failure.

The build `uv` is the normalized offscreen-pass UV. When showing a result as a
scene background, sample with screen UV instead:

```javascript
const screenUv = TSL.screenUV.flipY();
scene.backgroundNode = this.feedback.node.sample(screenUv).rgb;
```

Render targets, materials, nodes, and fallback textures are host-owned. Do not
mutate or dispose them. `clear()` queues both history targets to reset to
`clearValue`; `dispose()` is safe to call more than once. Shekere also disposes
all remaining passes when a sketch reloads, fails setup, switches, or unloads.

## Limits and scheduling

- Width and height: integer `1–1024`
- Live passes: at most `8` per sketch
- Logical pixels: at most `2,097,152` per sketch, before ping-pong duplication
- Format: `rgba8` by default; `rgba16f` only on a supported backend
- Filtering: linear, without mipmaps or color-space conversion

Passes run after camera bindings, camera motion analysis, and sketch `update`,
but before the main scene and post-processing render. A failing pass is
disabled independently and does not stop the sketch or other passes.

See [`camera_motion_ripple.js`](https://github.com/katk3n/shekere/blob/main/examples/camera_motion_ripple.js)
and [`feedback_particles.js`](https://github.com/katk3n/shekere/blob/main/examples/feedback_particles.js).
