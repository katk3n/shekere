export function setup(scene) {
  // 1. Load the image using TextureLoader
  const loader = new THREE.TextureLoader();
  const texture = loader.load(Shekere.convertFileSrc(Shekere.SKETCH_DIR + 'assets/shekere.png'));

  // 2. Create a plane to display the image
  const geometry = new THREE.PlaneGeometry(4, 4);
  const material = new THREE.MeshBasicMaterial({
    map: texture,
    side: THREE.DoubleSide // Display the image from both sides
  });

  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

// State variables for smooth transitions
let smoothRotationY = 0;
let smoothRotationZ = 0;
let smoothScale = 0;

export function update({ time, audio, midi }) {
  const lerp = (a, b, t) => a + (b - a) * t;

  // Control Y-axis rotation speed/angle with MIDI CC 1 (Mod Wheel)
  const targetRY = (midi.cc[1] ?? 0) * Math.PI * 2;
  smoothRotationY = lerp(smoothRotationY, targetRY, 0.1);

  // Control Z-axis tilt with MIDI CC 10 (Pan)
  const targetRZ = ((midi.cc[10] ?? 0.5) - 0.5) * 2; // -1.0 to 1.0
  smoothRotationZ = lerp(smoothRotationZ, targetRZ, 0.1);

  // Apply additional scale control with MIDI CC 11 (Expression)
  const targetS = (midi.cc[11] ?? 0);
  smoothScale = lerp(smoothScale, targetS, 0.1);

  // Add MIDI values to the base rotation
  this.mesh.rotation.y = (time * 0.3) + smoothRotationY;
  this.mesh.rotation.z = (Math.sin(time * 0.5) * 0.1) + (smoothRotationZ * 0.5);

  // Calculate scale from audio bass and MIDI CC 11
  const scale = 1 + (audio.bass * 0.5) + (smoothScale * 1.5);
  this.mesh.scale.set(scale, scale, scale);
}

export function cleanup(scene) {
  Shekere.clearScene(scene);
}
