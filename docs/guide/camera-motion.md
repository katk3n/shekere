# Camera Motion

Shekere can compare consecutive camera frames on the GPU and expose a current
motion mask and a decaying motion trail to sketches. This is useful for aura,
afterimage, reveal, distortion, displacement, and geometry-mask effects.

Motion analysis detects any image change. It is not pose estimation, arm
tracking, person segmentation, or object recognition. Camera movement,
lighting changes, exposure changes, and moving backgrounds also produce motion.

## Enabling analysis

Motion analysis is opt-in because it allocates GPU render targets and runs
several offscreen passes. Request it from the object returned by `setup(scene)`:

```javascript
export function setup(scene) {
  return {
    camera: {
      motion: {
        enabled: true,
        threshold: 0.08,
        blur: 6,
        decay: 0.94
      }
    }
  };
}
```

| Setting | Default | Range | Description |
| :--- | :--- | :--- | :--- |
| `enabled` | `false` | boolean | Enables GPU analysis for the active sketch. |
| `threshold` | `0.08` | `0.0–1.0` | Minimum luminance change treated as motion. |
| `blur` | `6` | `0–20` | Gaussian blur radius in analysis pixels. |
| `decay` | `0.94` | `0.0–0.999` | Amount of the previous trail retained per camera frame. |

Values outside their ranges are clamped. Omitting `camera.motion` or setting
`enabled: false` disables analysis and releases its GPU resources.

## Stable TSL nodes

TSL graphs are normally built during `setup(scene)`, before the first
`update(context)` call. Shekere therefore provides stable, host-owned nodes in
the global namespace:

```javascript
export function setup(scene) {
  const trail = Shekere.camera.motion.trailNode.sample(TSL.uv()).r;
  const color = TSL.uniform(new THREE.Color(0x35a7ff));

  this.material = new THREE.MeshBasicNodeMaterial();
  this.material.colorNode = color.mul(trail);
}
```

`maskNode` and `trailNode` keep the same identity while Shekere updates their
underlying textures. When analysis is inactive or initializing, they sample a
black fallback texture. Sketches do not need to create a fallback or rebind a
texture node after ping-pong swaps.

::: warning Node ownership
`Shekere.camera.motion.maskNode` and `trailNode`, including their fallback
texture, belong to Shekere. Sample them but do not assign their `value` or
dispose them.
:::

## Motion data

`camera.motion` is available in every `update(context)` call:

```javascript
export function update({ camera, audio, bloom }) {
  this.mesh.visible = camera.motion.active;
  bloom.strength = 0.5 + audio.bass * 3;
}
```

| Property | Type | Description |
| :--- | :--- | :--- |
| `active` | `boolean` | Whether completed motion textures are available. |
| `maskTexture` | `THREE.Texture \| null` | Blurred movement in the newest analyzed frame. |
| `trailTexture` | `THREE.Texture \| null` | Recent movement accumulated with temporal decay. |
| `width` | `number` | Analysis texture width. |
| `height` | `number` | Analysis texture height. |

The longest analysis dimension is 320 pixels. Analysis runs at most once per
new camera frame and occurs before the sketch update. The first frame clears
the history, so `active` remains false until a frame comparison is available.

`camera.motion` keeps the same object identity. Camera restart, device changes,
capture-dimension changes, and trail ping-pong updates may replace raw texture
references. Use the stable Shekere nodes for TSL graphs. Consumers that require
raw textures must refresh their references when they change.

::: warning Texture ownership
Motion textures belong to Shekere. Never dispose `maskTexture` or
`trailTexture` in a sketch. Dispose only resources created by the sketch.
:::

See [`examples/camera_motion_aura.js`](https://github.com/katk3n/shekere/blob/main/examples/camera_motion_aura.js)
for an audio-reactive TSL aura that samples the trail texture.

Effects that require particles with independent velocity and lifetime, growing
ripples, smoke, or iterative simulation state can use motion as an input to a
host-managed [GPU feedback pass](./gpu-feedback.md).
