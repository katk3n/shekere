# Camera

Shekere can expose a live camera feed to sketches as a Three.js
`VideoTexture`. Capture runs entirely in the Visualizer window, so video frames
are not serialized or transferred between windows.

## Starting the camera

Camera capture never starts automatically. In the Control Panel:

1. Select **Default Device** or a specific camera in the Camera section.
2. Click **Enable Camera** and allow camera access when prompted.
3. Confirm that the state changes to `active` and check the actual resolution
   and frame rate shown in the panel.

Device names may initially appear as generic labels until camera permission is
granted. Changing the selected device while capture is active restarts capture
with that exact device. If it cannot be opened, Shekere reports an error rather
than silently falling back to another camera.

## Sketch API

The `camera` property is available in every `update(context)` call:

```javascript
export function update({ camera }) {
  if (this.material.map !== camera.texture) {
    this.material.map = camera.texture;
    this.material.needsUpdate = true;
  }
}
```

| Property | Type | Description |
| :--- | :--- | :--- |
| `camera.active` | `boolean` | Whether live capture is active. |
| `camera.texture` | `THREE.VideoTexture \| null` | The current host-owned camera texture. |
| `camera.width` | `number` | Actual capture width in pixels. |
| `camera.height` | `number` | Actual capture height in pixels. |
| `camera.frameRate` | `number` | Actual capture frame rate reported by the device. |

The `camera` object keeps the same identity between frames. When capture is
inactive or has failed, `active` is `false`, `texture` is `null`, and all
numeric values are `0`.

Restarting capture or changing devices can replace `camera.texture`. Sketches
must compare and update their material texture reference as shown above.

::: warning Texture ownership
The `VideoTexture` belongs to Shekere. Never call `camera.texture.dispose()` in
a sketch. During cleanup, dispose only geometry, materials, and textures that
the sketch created itself.
:::

Camera capture is independent of sketch lifecycle. Reloading or switching a
sketch does not stop an active camera. Click **Stop Camera** when capture is no
longer needed.

## Capture defaults and troubleshooting

Shekere requests 1280×720 at 30 fps as preferred values. Cameras may choose a
different supported format; the actual values are exposed to the sketch and
shown in the Control Panel.

The Control Panel distinguishes permission denial, missing or disconnected
devices, unsupported constraints, unavailable browser media APIs, and other
capture or playback failures. Camera failure leaves `context.camera` inactive
and does not stop the Visualizer render loop.
