export function setup(scene) {
  const geometry = new THREE.TorusKnotGeometry(1, 0.3, 100, 16);
  // MeshNormalMaterial is nice because it doesn't need lights
  const material = new THREE.MeshNormalMaterial();

  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

export function update(context) {
  // Rotate smoothly over time
  this.mesh.rotation.x = context.time * 1.0;
  this.mesh.rotation.y = context.time * 1.0;
}

export function cleanup(scene) {
  // Important for hot reloading: remove and dispose of geometries/materials
  scene.remove(this.mesh);
  this.mesh.geometry.dispose();
  this.mesh.material.dispose();
}
