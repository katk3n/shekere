# 0004: Meyda.js Audio Feature Extraction

## Status
Implemented (v0.8.0)

## Context
Shekere currently processes audio via the Web Audio API, extracting basic frequency bands (FFT) and overall volume. While this is sufficient for simple audio-reactivity, modern VJ performances often require more semantic music analysis, such as detecting percussive transients versus sustained tones, or determining the "brightness" of a sound to control color palettes dynamically.

To enable advanced sketch visualizations without violating ADR `0001` (Audio processing must remain in the Web Audio API layer, not in Rust), we need a JavaScript-based audio feature extraction library.

Meyda.js is selected as the most suitable library because it:
1. Is lightweight and runs entirely in JavaScript.
2. Integrates seamlessly with the Web Audio API in real-time.
3. Provides essential VJ features (RMS, Zero-Crossing Rate, Spectral Centroid, etc.) without the heavy overhead of complete MIR (Music Information Retrieval) libraries like Essentia.js.

## Decision

1. **Default Feature Extraction**  
   To provide a superior Developer Experience (DX) and ensure the Control Panel's visualizers (Chroma, MFCC) are always active, Meyda.js is configured to extract core features (**RMS, ZCR, Energy, Spectral Centroid, Spectral Flatness, Chroma, MFCC**) by default on every frame.

   Sketches no longer need to explicitly declare their required features in the `setup` function. They can access any of the core features directly from the `context.audio.features` object.

   ```javascript
   export function setup(scene) {
       // No absolute need for audio features declaration anymore
       return {};
   }
   ```

2. **API Injection into `update`**  
   The requested features will be injected into the `context.audio.features` object provided to the sketch's `update` function on every frame.

   ```javascript
   export function update(context) {
       const { zcr } = context.audio.features || {};
       if (zcr > 50) { /* trigger visual effect */ }
   }
   ```

3. **Buffer Size Standardization**  
   To ensure temporal and spectral consistency between the existing basic audio bands and Meyda.js features, both will use the same buffer size (`FFT_SIZE = 4096`).

## Available Features (Specifications)

Sketches can request any valid Meyda feature. Below are the primary features that will become available and their expected use cases in Shekere:

- **`rms` (Root Mean Square)**  
  - **Type:** `number`
  - **Description:** An accurate representation of human-perceived loudness, smoother than raw amplitude.
  - **Use Case:** Global scaling of meshes, intensity of Bloom, or overall scene brightness.
- **`zcr` (Zero-Crossing Rate)**  
  - **Type:** `number`
  - **Description:** The rate at which the signal fluctuates between positive and negative values. High values indicate noisy or percussive sounds (e.g., hi-hats, snares).
  - **Use Case:** Triggering sudden events, glitches, or flashes specifically on drum hits rather than melodic notes.
- **`energy`**
  - **Type:** `number`
  - **Description:** The sum of the squares of the signal values. Represents the raw acoustic energy.
- **`spectralCentroid` (Brightness)**  
  - **Type:** `number`
  - **Description:** The "center of mass" of the frequency spectrum. A higher centroid means the sound feels brighter or higher-pitched.
  - **Use Case:** Mapping to the HSL hue (e.g., bass sounds render warm colors, high sounds render cool colors) or controlling camera Z-depth based on sound brightness.
- **`spectralFlatness`**  
  - **Type:** `number` (typically 0.0 to 1.0 area)
  - **Description:** Indicates whether a sound is tonelike (close to 0) or noiselike (close to 1).
  - **Use Case:** Controlling particle turbulence, dispersion rate, or shader noise intensity.
- **`chroma`**
  - **Type:** `number[12]` (Array of 12 numbers)
  - **Description:** Chromagram representing the energy distribution across the 12 pitch classes (C, C#, D, etc.).
  - **Use Case:** Driving visuals based on specific musical keys or chords playing.
- **`mfcc` (Mel-Frequency Cepstral Coefficients)**
  - **Type:** `number[13]` (Array of 13 numbers)
  - **Description:** Represents the shape of the spectral envelope (timbre/vocal tract shape).
  - **Use Case:** Complex shader parameter modulation or mapping to 3D terrain deformation.


## Consequences

**Positive:**
- Sketches can utilize high-level audio descriptors (like brightness and noisiness) to create sophisticated visual mappings.
- Performance remains optimal since features are only extracted when explicitly requested.
- Preserves the existing architecture by keeping heavy processing out of the Rust layer.

**Negative / Risks:**
- A very large array of requested features in a Sketch may slightly increase the CPU load on the browser's main thread. Users scaling up their sketch complexity will need to monitor performance.
