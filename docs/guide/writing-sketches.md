# Writing Sketches

Shekere allows you to write visuals using JavaScript. This guide explains the core API and how to interact with the visualizer.

## The Sketch API

Every Shekere sketch must export an `update` function. This function is called on every frame (usually 60 times per second) to render your visual.

```javascript
/**
 * @param {CanvasRenderingContext2D} ctx - The 2D rendering context
 * @param {number} width - The current width of the visualizer window
 * @param {number} height - The current height of the visualizer window
 * @param {object} audio - The audio analysis data provided by Meyda
 */
export function update(ctx, width, height, audio) {
  // Your drawing logic here
}
```

### 1. The Context (`ctx`)
Shekere provides a standard `CanvasRenderingContext2D`. You can use all standard Canvas API methods like `fillRect()`, `stroke()`, `beginPath()`, etc.

### 2. Dimensions (`width` and `height`)
These values represent the size of the visualizer window. Shekere automatically handles window resizing, so you should always use these variables to position your elements.

### 3. Audio Data (`audio`)
The `audio` object contains real-time analysis data extracted using the [Meyda](https://meyda.js.org/) library. Common properties include:

| Property | Type | Description |
| :--- | :--- | :--- |
| `rms` | `number` | Root Mean Square (perceived loudness). Ranges roughly from 0.0 to 1.0. |
| `energy` | `number` | The total energy of the audio signal. |
| `zcr` | `number` | Zero Crossing Rate (helps detect noise vs tones). |
| `amplitudeSpectrum` | `Float32Array` | The magnitude of each frequency band (FFT data). |
| `complexSpectrum` | `object` | Real and imaginary parts of the FFT. |

Example using frequency data:
```javascript
export function update(ctx, width, height, audio) {
  const bands = audio.amplitudeSpectrum;
  const barWidth = width / bands.length;

  ctx.fillStyle = 'white';
  for (let i = 0; i < bands.length; i++) {
    const barHeight = bands[i] * height;
    ctx.fillRect(i * barWidth, height - barHeight, barWidth, barHeight);
  }
}
```

## State Management

Since the `update` function is called every frame, any variables declared *inside* the function will be reset. To maintain state (like a counter or an object's position) across frames, declare them *outside* the function.

```javascript
let rotation = 0;

export function update(ctx, width, height, audio) {
  rotation += audio.rms * 0.1; // Rotate based on volume

  ctx.save();
  ctx.translate(width / 2, height / 2);
  ctx.rotate(rotation);
  ctx.fillStyle = 'red';
  ctx.fillRect(-50, -50, 100, 100);
  ctx.restore();
}
```

## Tips for Better Visuals

1.  **Normalization**: Most audio features are raw numbers. You often need to multiply them by a constant (e.g., `audio.rms * 500`) to get useful values for drawing.
2.  **Smoothing**: Audio data can be jumpy. Consider using a simple easing function to smooth out the movement:
    `targetSize = audio.rms * 1000; currentSize += (targetSize - currentSize) * 0.1;`
3.  **Alpha Blending**: Instead of clearing the screen with `fillRect`, try drawing a semi-transparent rectangle to create a trail effect:
    `ctx.fillStyle = 'rgba(0, 0, 0, 0.1)'; ctx.fillRect(0, 0, width, height);`
