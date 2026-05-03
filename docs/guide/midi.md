# MIDI

Shekere automatically detects connected MIDI devices and provides real-time data for notes and control changes (CC).

## The `midi` Object

The `midi` object contains two arrays, each with 128 elements. All values are normalized to a range of **0.0 to 1.0**.

| Property | Description |
| :--- | :--- |
| `midi.notes` | Array of note velocities. `midi.notes[60]` is Middle C. |
| `midi.cc` | Array of Control Change values (knobs, faders). |

### Note Data
- When a key is pressed (Note On), the value in the array becomes the velocity (how hard it was hit) divided by 127.
- When released (Note Off), the value becomes `0`.

### CC Data
- Knobs and sliders update the corresponding index in the `midi.cc` array from `0.0` to `1.0`.

## Example Usage

### Mapping a Controller to FX
Using a slider (usually CC 74 on many controllers) to control the Bloom strength:

```javascript
export function update({ midi, bloom }) {
  // Directly map CC 74 (0.0 - 1.0) to bloom strength
  bloom.strength = midi.cc[74] * 3.0;
}
```

### Triggering Visuals with Keys
Flashing a color when the lowest C (Note 36) is pressed:

```javascript
export function update({ midi }) {
  const velocity = midi.notes[36];
  if (velocity > 0) {
    this.colorIntensity = velocity;
  }
  this.colorIntensity *= 0.95; // Fade out
}
```

## Tips for MIDI

1.  **Normalization**: Since Shekere values are always 0.0 - 1.0, you don't need to manually divide MIDI data by 127.
2.  **Smoothing**: MIDI controllers often send data at a lower frequency than 60fps, which can cause "stepping" in visuals. Consider smoothing CC values:
    `currentValue += (midi.cc[10] - currentValue) * 0.1;`
3.  **Debugging**: If you are unsure which CC or Note number your hardware is sending, check the **Monitors** section in the Shekere Control Panel. It visually displays active MIDI signals in real-time.
