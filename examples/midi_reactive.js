/**
 * Colored MIDI Starry Night (No Moon)
 * 
 * - Each MIDI note triggers stars of specific colors.
 * - CC 7: Controls background star brightness.
 * - CC 10: Star field tilt.
 */

const STAR_COUNT = 1500;
let stars;
let starData = [];

export function setup(scene) {
  // 1. Instanced Stars with Colors
  const starGeometry = new THREE.SphereGeometry(0.1, 8, 8);
  const starMaterial = new THREE.MeshBasicMaterial({ color: 0xffffff }); // Base white
  stars = new THREE.InstancedMesh(starGeometry, starMaterial, STAR_COUNT);
  
  const dummy = new THREE.Object3D();
  const color = new THREE.Color();
  
  for (let i = 0; i < STAR_COUNT; i++) {
    const r = 20 + Math.random() * 40;
    const theta = Math.random() * Math.PI * 2;
    const phi = Math.acos(2 * Math.random() - 1);
    
    dummy.position.set(
      r * Math.sin(phi) * Math.cos(theta),
      r * Math.sin(phi) * Math.sin(theta),
      r * Math.cos(phi)
    );
    
    dummy.updateMatrix();
    stars.setMatrixAt(i, dummy.matrix);
    
    // Assign random vibrant colors
    color.setHSL(Math.random(), 0.7, 0.6);
    stars.setColorAt(i, color);
    
    starData.push({
      noteIndex: i % 128, 
      baseScale: 0.1 + Math.random() * 0.4,
      twinkleSpeed: 1 + Math.random() * 3,
      phase: Math.random() * Math.PI * 2
    });
  }
  scene.add(stars);

  // 2. Subtle background light
  const ambient = new THREE.AmbientLight(0x101020);
  scene.add(ambient);
}

export function update({ time, audio, midi }) {
  const dummy = new THREE.Object3D();
  const bgBrightness = midi.cc[7] || 0.2;
  
  for (let i = 0; i < STAR_COUNT; i++) {
    const data = starData[i];
    stars.getMatrixAt(i, dummy.matrix);
    dummy.matrix.decompose(dummy.position, dummy.quaternion, dummy.scale);
    
    const velocity = midi.notes[data.noteIndex] || 0;
    const twinkling = Math.sin(time * data.twinkleSpeed + data.phase) * 0.1 + 0.9;
    
    // MIDI Note boost makes them much larger and brighter
    const s = data.baseScale * (twinkling + bgBrightness + velocity * 10.0);
    
    dummy.scale.set(s, s, s);
    dummy.updateMatrix();
    stars.setMatrixAt(i, dummy.matrix);
  }
  stars.instanceMatrix.needsUpdate = true;

  const tilt = (midi.cc[10] || 0.5) - 0.5;
  stars.rotation.y = time * 0.03 + tilt * 2.0;
  stars.rotation.x = time * 0.01;
}

export function cleanup(scene) {
  scene.remove(stars);
  if (stars.geometry) stars.geometry.dispose();
  if (stars.material) stars.material.dispose();
}
