/**
 * spectrum.js - 256-band Audio Spectrum Visualizer
 *
 * Renders a full-frequency bar chart using FFT data from the microphone.
 * Bars grow upward from the bottom of the screen; color shifts from blue
 * (low frequencies, left) to red (high frequencies, right).
 *
 * Frequency range: 80 Hz (Human voice low) to 2,000 Hz (Human voice high)
 * Divided into 256 logarithmic bands - one bar per band.
 *
 * Camera assumptions: PerspectiveCamera at z=5, FOV=75.
 * Adjust VISIBLE_WIDTH and BOTTOM_Y if you change the camera.
 */

const BAND_COUNT = 256;
const MAX_HEIGHT = 7.5; // Almost the full screen height
// Bottom Y coordinate (Assuming z=5, FOV=75, is approx. -3.8)
const BOTTOM_Y = -3.8;
// Spread across the full width (Slightly larger than actual visible width)
const VISIBLE_WIDTH = 13.0; 
const BAR_SLOT   = VISIBLE_WIDTH / BAND_COUNT;
const BAR_WIDTH  = BAR_SLOT * 0.8;
const BAR_SPACING = BAR_SLOT * 0.2;
const TOTAL_WIDTH = BAND_COUNT * BAR_SLOT;

export function setup(scene) {
  this.bars = [];
  this.geometry = new THREE.BoxGeometry(BAR_WIDTH, 1, BAR_WIDTH);

  for (let i = 0; i < BAND_COUNT; i++) {
    const x = i * BAR_SLOT - TOTAL_WIDTH / 2 + BAR_WIDTH / 2;

    // Gradient based on frequency (Low: Blue, Mid: Green, High: Red)
    const t = i / (BAND_COUNT - 1);
    const color = new THREE.Color().setHSL(0.66 - t * 0.66, 1.0, 0.55);

    const material = new THREE.MeshStandardMaterial({ color, roughness: 0.4, metalness: 0.3 });
    const bar = new THREE.Mesh(this.geometry, material);

    bar.position.set(x, BOTTOM_Y, 0);
    bar.scale.set(1, 0.01, 1);
    scene.add(bar);
    this.bars.push(bar);
  }

  // Lights
  this.ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
  scene.add(this.ambientLight);

  this.pointLight = new THREE.PointLight(0xffffff, 60, 50);
  this.pointLight.position.set(0, 5, 5); // Adjust position
  scene.add(this.pointLight);

  // Return audio analysis configuration (for human voice: 80Hz - 2000Hz)
  return {
    audio: {
      minFreqHz: 80,
      maxFreqHz: 2000,
    }
  };
}

export function update({ time, audio }) {
  const { bands, volume } = audio;

  for (let i = 0; i < BAND_COUNT; i++) {
    const bar = this.bars[i];
    const targetHeight = Math.max(0.01, bands[i] * MAX_HEIGHT);

    // Smoothly follow (lerp)
    bar.scale.y += (targetHeight - bar.scale.y) * 0.25;

    // Align base of bars with BOTTOM_Y
    bar.position.y = BOTTOM_Y + bar.scale.y / 2;

    // Brightness changes with overall volume
    bar.material.emissiveIntensity = volume * 0.5;
    bar.material.emissive = bar.material.color;
  }

  // Slowly move point light over time
  this.pointLight.position.x = Math.sin(time * 0.3) * 15;
}

export function cleanup(scene) {
  for (const bar of this.bars) {
    scene.remove(bar);
    bar.material.dispose(); // Material is unique to each bar
  }
  this.geometry.dispose(); // Geometry is shared
  scene.remove(this.ambientLight);
  scene.remove(this.pointLight);
}
