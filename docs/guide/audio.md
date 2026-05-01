# Audio

Shekere processes audio in real-time, providing both simple volume levels and deep spectral features to your sketches.

## Basic Audio Properties

The `audio` object passed to the `update` function contains normalized values (0.0 to 1.0) representing different frequency ranges.

| Property | Description |
| :--- | :--- |
| `volume` | The overall average loudness of the signal. |
| `bass` | Average energy in the low frequencies (up to 250 Hz). |
| `mid` | Average energy in the mid frequencies (250 Hz - 2000 Hz). |
| `high` | Average energy in the high frequencies (above 2000 Hz). |

### Example: Basic Reactivity
```javascript
export function update({ audio }) {
  // Use bass to control the size of a sphere
  const scale = 1 + audio.bass * 2;
  this.mesh.scale.set(scale, scale, scale);
}
```

## Frequency Bands (FFT)

`audio.bands` is a `Float32Array` of **256 bins**, representing the frequency spectrum from low to high. The scaling is logarithmic to better match human hearing.

### Example: Spectrum Visualizer
```javascript
export function update({ ctx, width, height, audio }) {
  const barWidth = width / audio.bands.length;
  ctx.fillStyle = 'white';
  
  audio.bands.forEach((value, i) => {
    const barHeight = value * height;
    ctx.fillRect(i * barWidth, height - barHeight, barWidth, barHeight);
  });
}
```

## Advanced Features (Meyda)

For more sophisticated analysis, use the `audio.features` object. These are powered by the Meyda library.

| Feature | Type | Use Case |
| :--- | :--- | :--- |
| `rms` | `number` | Root Mean Square. More accurate perceived loudness than `volume`. |
| `zcr` | `number` | Zero-Crossing Rate. Useful for detecting percussive/noise-like sounds. |
| `energy` | `number` | The total acoustic energy of the signal. |
| `spectralCentroid` | `number` | The "center of mass" of the spectrum. Indicates the "brightness" of the sound. |
| `spectralFlatness` | `number` | Distinguishes between pure tones (0.0) and noise (1.0). |
| `chroma` | `number[12]` | Intensity of the 12 pitch classes (C, C#, D, etc). Useful for reacting to harmony/melody. |
| `mfcc` | `number[13]` | Mel-Frequency Cepstral Coefficients. Represents timbre or spectral shape. |

### Example: Percussion Detection
```javascript
export function update({ audio }) {
  // Trigger a flash if the sound is very percussive (high ZCR)
  if (audio.features.zcr > 50) {
    this.flash = 1.0;
  }
  this.flash *= 0.9; // Decay the flash
}
```

## Configuration

You can customize the frequency range analyzed by returning an `audio` object in your `setup` function.

```javascript
export function setup(scene) {
  return {
    audio: {
      minFreqHz: 40,   // Default: 27.5
      maxFreqHz: 8000  // Default: 4186
    }
  };
}
```
