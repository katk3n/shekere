/**
 * audio_reactive_knot.js — Audio-Reactive TorusKnot
 *
 * A minimal example showing how to combine time-based animation with
 * real-time microphone data.
 *
 * Behavior:
 *   - The knot rotates continuously over time.
 *   - Its scale grows with bass energy (low frequencies).
 *   - Wireframe mode activates when high-frequency energy exceeds 0.3.
 *
 * Audio data used:
 *   context.audio.bass   low-frequency energy  (0.0 – 1.0)
 *   context.audio.high   high-frequency energy (0.0 – 1.0)
 *
 * Enable the microphone in the Control Panel to hear it react to sound.
 */

export function setup(scene) {
  const geometry = new THREE.TorusKnotGeometry(1, 0.3, 100, 16);
  // MeshNormalMaterial is nice because it doesn't need lights
  const material = new THREE.MeshNormalMaterial({ wireframe: false });

  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

export function update(context) {
  const { time, audio } = context;

  // Rotate smoothly over time
  this.mesh.rotation.x = time * 0.5;
  this.mesh.rotation.y = time * 0.7;

  // Scale reacts to bass (low frequency)
  const bassScale = 1.0 + audio.bass * 2.0;
  this.mesh.scale.set(bassScale, bassScale, bassScale);

  // Wireframe toggles on high frequency
  this.mesh.material.wireframe = audio.high > 0.3;
}

export function cleanup(scene) {
  // Important for hot reloading: remove and dispose of geometries/materials
  scene.remove(this.mesh);
  this.mesh.geometry.dispose();
  this.mesh.material.dispose();
}
