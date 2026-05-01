# Effects (Post-Processing)

Shekere includes a powerful post-processing pipeline that can be controlled via the **Control Panel UI** or directly from your **Sketch Code**.

## Bidirectional Sync

One of Shekere's unique features is **Bidirectional Sync**. 
- If you move a slider in the Control Panel, the corresponding value in your code changes.
- If you change a value in your code (e.g., `bloom.strength = 1.5`), the slider in the Control Panel moves automatically to match.

This allows for a seamless workflow between manual tweaking and code-driven automation.

## Available Effects

You can access these effects from the `context` object passed to the `update` function.

### 🌸 Bloom (Glow)
| Property | Range | Default | Description |
| :--- | :--- | :--- | :--- |
| `bloom.strength` | 0.0 – 3.0 | `0.0` | Overall intensity of the glow. |
| `bloom.radius` | 0.0 – 1.0 | `0.0` | Blur radius of the bloom. |
| `bloom.threshold` | 0.0 – 1.0 | `1.0` | Brightness threshold. Only pixels brighter than this will glow. |

### 🌈 RGB Shift
| Property | Range | Default | Description |
| :--- | :--- | :--- | :--- |
| `rgbShift.amount` | 0.0 – 0.05 | `0.0` | Color channel offset amount (chromatic aberration). |

### 🎞️ Film Grain
| Property | Range | Default | Description |
| :--- | :--- | :--- | :--- |
| `film.intensity` | 0.0 – 2.0 | `0.0` | Intensity of the noise grain. |

### 🎭 Vignette
| Property | Range | Default | Description |
| :--- | :--- | :--- | :--- |
| `vignette.offset` | 0.0 – 3.0 | `0.0` | Radius of the vignette. |
| `vignette.darkness` | 0.0 – 3.0 | `1.0` | Intensity/Blackness of the edges. |

## Example Usage

```javascript
export function update({ audio, bloom, rgbShift, film }) {
  // Make the scene glow based on bass
  bloom.strength = audio.bass * 2.0;

  // Add glitchy RGB shift on high frequencies
  rgbShift.amount = audio.high * 0.01;

  // Static film grain intensity
  film.intensity = 0.5;
}
```

## Performance Note

To save GPU power, Shekere automatically disables post-processing passes when their intensity is set to `0`. If your sketch doesn't use any effects, there is no performance penalty.
