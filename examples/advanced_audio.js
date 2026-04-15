/**
 * advanced_audio.js - Advanced Audio Features Showcase
 * 
 * A comprehensive visualization demonstrating all Meyda.js audio features 
 * (RMS, ZCR, Centroid, Flatness, Chroma, and MFCC) mapped to visual elements.
 */

export function setup(scene) {
  this.orbs = [];
  this.group = new THREE.Group();
  scene.add(this.group);

  const radius = 4; // Circle radius
  
  for (let i = 0; i < 12; i++) {
    // Offset angle so 0 (C) is at the top
    const angle = (i / 12) * Math.PI * 2 + Math.PI / 2;
    
    const geometry = new THREE.SphereGeometry(0.5, 32, 32);
    const color = new THREE.Color().setHSL(i / 12, 0.8, 0.5);
    const material = new THREE.MeshStandardMaterial({
      color: color,
      emissive: color,
      emissiveIntensity: 0,
      metalness: 0.5,
      roughness: 0.2
    });

    const orb = new THREE.Mesh(geometry, material);
    // Use negative cos/sin to arrange naturally in clockwise circle
    orb.position.set(Math.cos(angle) * radius, Math.sin(angle) * radius, 0);
    
    this.group.add(orb);
    this.orbs.push(orb);
  }

  // Add a soft center light
  this.centerLight = new THREE.PointLight(0xffffff, 2, 10);
  scene.add(this.centerLight);

  return {};
}

export function update({ time, audio, bloom }) {
  const features = audio.features || {};
  const chroma = features.chroma || new Array(12).fill(0);
  const mfcc = features.mfcc || new Array(13).fill(0);
  
  let rms = features.rms || 0;
  let zcr = features.zcr || 0;
  let centroid = features.spectralCentroid || 0;
  let flatness = features.spectralFlatness || 0;

  // --- Tuning: Noise Gate ---
  const noiseFloor = 0.01;
  if (rms < noiseFloor) {
    rms = 0; zcr = 0; centroid = 0; flatness = 0;
  } else {
    rms = (rms - noiseFloor) * 1.2;
  }

  // --- 1. Spectral Centroid (Brightness) -> Center Light Color & Rotation ---
  this.smoothCentroid = THREE.MathUtils.lerp(this.smoothCentroid || 0, centroid, 0.1);
  const normalizedCentroid = Math.min(Math.max(this.smoothCentroid / 6000.0, 0.0), 1.0);
  if (this.centerLight) {
    this.centerLight.color.setHSL(normalizedCentroid, 0.8, 0.5);
  }

  // --- 2. RMS / Energy -> Global Scale ---
  const globalScale = 1.0 + rms * 2.5;
  this.group.scale.lerp(new THREE.Vector3(globalScale, globalScale, globalScale), 0.15);

  const radiusBase = 4;

  this.orbs.forEach((orb, i) => {
    // --- 3. Chroma -> Orb Glow & Base Scale ---
    const val = chroma[i] || 0;
    const gate = rms > 0.02 ? 1 : 0;
    const intensity = Math.pow(val, 2) * gate;
    
    // Target scale and smoothing
    const targetScale = 0.5 + intensity * 1.5;
    orb.scale.lerp(new THREE.Vector3(targetScale, targetScale, targetScale), 0.1);
    
    // Smooth emissive intensity for more subtle glow
    const targetEmissive = intensity * 3.0;
    orb.material.emissiveIntensity = THREE.MathUtils.lerp(orb.material.emissiveIntensity, targetEmissive, 0.1);

    // --- 4. MFCC -> Radial Orbit Jitter ---
    // MFCC values typically range around -20 to 20
    const mfccVal = (mfcc[i] || 0) * 0.03; 

    // --- 5. Spectral Flatness (Noisiness) -> Chaotic Scatter ---
    const scatter = (Math.random() - 0.5) * flatness * 3.0;

    // Calculate position
    const angle = (i / 12) * Math.PI * 2 + Math.PI / 2;
    const currentRadius = radiusBase + mfccVal + scatter;
    
    const targetPos = new THREE.Vector3(
      Math.cos(angle) * currentRadius,
      Math.sin(angle) * currentRadius,
      0
    );
    orb.position.lerp(targetPos, 0.15);
  });

  // Base rotation influenced by brightness (Centroid)
  this.group.rotation.z -= (0.005 + normalizedCentroid * 0.02);

  // --- 6. ZCR -> Percussive Hit Glitch & Bloom Flash ---
  this.targetBloom = this.targetBloom || 0;
  if (rms > 0.05 && zcr > 150) {
    this.targetBloom = 1.8;
    
    // Glitch rotation
    this.group.rotation.x += (Math.random() - 0.5) * 0.3;
    this.group.rotation.y += (Math.random() - 0.5) * 0.3;
  } else {
    this.targetBloom *= 0.85;
    
    // Stabilize rotation back to flat slowly
    this.group.rotation.x = THREE.MathUtils.lerp(this.group.rotation.x, 0, 0.1);
    this.group.rotation.y = THREE.MathUtils.lerp(this.group.rotation.y, 0, 0.1);
  }

  // More conservative bloom settings
  bloom.strength = 0.4 + this.targetBloom;
  bloom.threshold = 0.2;
  bloom.radius = 0.8;
}

export function cleanup(scene) {
  Shekere.clearScene(scene);
}
