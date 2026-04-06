/**
 * hello_world.js - Minimal Starting Template
 * 
 * A basic example showing the core lifecycle functions: 
 * setup, update, and cleanup.
 */

export function setup(scene) {
  // One-time setup: Add a wireframe icosahedron to the scene
  const geometry = new THREE.IcosahedronGeometry(1, 2);
  const material = new THREE.MeshNormalMaterial({ wireframe: true });
  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

export function update({ time, audio }) {
  // Animate: Rotate over time
  this.mesh.rotation.y = time * 0.5;
  
  // React: Scale based on bass energy (low frequencies)
  const s = 1 + audio.bass;
  this.mesh.scale.set(s, s, s);
}

export function cleanup(scene) {
  // Cleanup: Remove objects and dispose resources when switching sketches
  scene.remove(this.mesh);
  this.mesh.geometry.dispose();
  this.mesh.material.dispose();
}
