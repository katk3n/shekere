/**
 * hello_world.js - Minimal Starting Template
 */

export function setup(scene) {
  const geometry = new THREE.IcosahedronGeometry(1, 2);
  const material = new THREE.MeshNormalMaterial({ wireframe: true });
  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

export function update({ time, audio }) {
  // Rotate smoothly over time
  this.mesh.rotation.y = time * 0.5;
  
  // React to bass (bass energy from 0.0 to 1.0)
  const s = 1 + audio.bass;
  this.mesh.scale.set(s, s, s);
}

export function cleanup(scene) {
  Shekere.clearScene(scene);
}
