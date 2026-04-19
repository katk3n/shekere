/**
 * advanced_audio.js - Advanced Audio Visualization Template
 * 
 * This example demonstrates how to map high-level audio features to 3D elements:
 * 1. Global Scalar Features (RMS, Centroid, ZCR, Flatness) mapped to a central crystal.
 * 2. Array Features (Chroma, MFCC) mapped to a swarm of orbiting orbs.
 * 
 * Features used:
 * - RMS: Overall volume levels.
 * - Spectral Centroid: Perceived brightness of the sound.
 * - ZCR & Transients: Sharp/percussive hits.
 * - Flatness: Tonality vs. Noise.
 * - Chroma: 12-semitone musical pitch intensity.
 * - MFCC: Timbral characteristics (envelope).
 */

export function setup(scene) {
  this.group = new THREE.Group();
  scene.add(this.group);

  // --- 1. Central Crystal (Scalar Audio Features) ---
  // A multi-faceted polyhedral object that reacts to the overall energy and tone.
  const crystalGeo = new THREE.IcosahedronGeometry(0.7, 0); 
  const crystalMat = new THREE.MeshStandardMaterial({
    color: 0xffffff,
    emissive: 0xffffff,
    emissiveIntensity: 0.02,
    metalness: 0.8,
    roughness: 0.2,
    flatShading: true
  });
  this.centerCrystal = new THREE.Mesh(crystalGeo, crystalMat);
  this.group.add(this.centerCrystal);

  // --- 2. Orbiting Orbs (Array Audio Features: Chroma & MFCC) ---
  // 12 orbs representing the chromatic scale (C, C#, D, etc.).
  this.orbs = [];
  this.baseRadius = 2.5; 
  
  for (let i = 0; i < 12; i++) {
    const angle = (i / 12) * Math.PI * 2 + Math.PI / 2;
    
    // Geometry size is optimized for screen visibility
    const orbGeo = new THREE.SphereGeometry(0.18, 32, 32);
    const orbColor = new THREE.Color().setHSL(i / 12, 0.8, 0.5);
    const orbMat = new THREE.MeshStandardMaterial({
      color: orbColor,
      emissive: orbColor,
      emissiveIntensity: 0,
      metalness: 0.5,
      roughness: 0.2
    });

    const orb = new THREE.Mesh(orbGeo, orbMat);
    orb.position.set(Math.cos(angle) * this.baseRadius, Math.sin(angle) * this.baseRadius, 0);
    
    this.group.add(orb);
    this.orbs.push(orb);
  }

  // --- 3. Lighting Configuration ---
  // A dynamic triple-lighting setup ensures 3D depth and facet definition.
  
  // Main reactive light (follows crystal emission)
  this.centerLight = new THREE.PointLight(0xffffff, 2, 20);
  this.centerLight.position.set(5, 5, 8);
  scene.add(this.centerLight);
  
  // Static directional light to maintain silhouette and shadows
  const sideLight = new THREE.DirectionalLight(0xffffff, 0.8);
  sideLight.position.set(-5, 2, 2);
  scene.add(sideLight);

  // Subtle rear rim light
  const rimLight = new THREE.PointLight(0xffffff, 0.5, 10);
  rimLight.position.set(0, 0, -5);
  scene.add(rimLight);

  // Base ambient light for baseline visibility
  const ambientLight = new THREE.AmbientLight(0xffffff, 0.1);
  scene.add(ambientLight);

  // --- 4. Initialization of Analysis State ---
  this.prevRms = 0;
  this.targetBloom = 0;

  return {};
}

export function update({ time, audio, bloom }) {
  // Retrieve features extracted by the core visualizer
  const features = audio.features || {};
  
  // Scalar Data
  let rms = features.rms || 0;
  let zcr = features.zcr || 0;
  let centroid = features.spectralCentroid || 0;
  let flatness = features.spectralFlatness || 0;
  
  // Array Data (Chroma = 12 bins, MFCC = 13 bins)
  const chroma = features.chroma || new Array(12).fill(0);
  const mfcc = features.mfcc || new Array(13).fill(0);

  // Apply noise floor gate for cleaner visuals in quiet sections
  const noiseFloor = 0.01;
  if (rms < noiseFloor) rms = 0;
  else rms = (rms - noiseFloor) * 1.2;

  // --- A. CENTRAL CRYSTAL MAPPINGS ---

  // 1. RMS (Volume) -> Base Scale
  const baseScale = 1.0 + rms * 1.5;

  // 2. Spectral Centroid (Brightness) -> Color Hue
  // Normalization divisor (1000) is tuned for standard microphone/music input.
  // Whistling or sharp transients push this towards cooler colors (blue/purple).
  this.smoothCentroid = THREE.MathUtils.lerp(this.smoothCentroid || 0, centroid, 0.2);
  const normalizedCentroid = Math.min(Math.max(this.smoothCentroid / 1000.0, 0.0), 1.0);
  const hue = normalizedCentroid;
  this.centerCrystal.material.color.setHSL(hue, 1.0, 0.5);
  this.centerCrystal.material.emissive.setHSL(hue, 1.0, 0.5);
  
  // 3. Spectral Flatness (Noisiness) -> Continuous Rotation Speed
  this.centerCrystal.rotation.x += 0.002 + flatness * 0.1;
  this.centerCrystal.rotation.y += 0.003 + flatness * 0.15;

  // 4. ZCR & RMS Delta (Percussion) -> Pop Effect
  // Detects sudden energy spikes combined with high-frequency noise.
  const rmsDelta = rms - this.prevRms;
  const isPercussiveHit = rmsDelta > 0.015 && zcr > 40 && flatness > 0.2;

  if (isPercussiveHit) {
    this.targetBloom = 3.5;
    const peakScale = baseScale * 1.4; 
    this.centerCrystal.scale.set(peakScale, peakScale, peakScale);
    this.centerCrystal.material.emissiveIntensity = 6.0;
  } else {
    this.targetBloom *= 0.8;
    // Slower lerp for scale and emissive provides a more natural visual 'decay'
    this.centerCrystal.scale.lerp(new THREE.Vector3(baseScale, baseScale, baseScale), 0.05);
    this.centerCrystal.material.emissiveIntensity = THREE.MathUtils.lerp(this.centerCrystal.material.emissiveIntensity, 0.02, 0.05);
  }

  // Update light intensity based on the hit flash
  this.centerLight.color.copy(this.centerCrystal.material.color);
  this.centerLight.intensity = this.centerCrystal.material.emissiveIntensity * 3;


  // --- B. ORBITING ORB MAPPINGS ---

  this.orbs.forEach((orb, i) => {
    // 1. Chroma (Note intensity) -> Individual Glow
    // Squared values increase contrast, highlighting the dominant notes of a chord.
    const noteIntensity = Math.pow(chroma[i] || 0, 2);
    const activeGlow = rms > 0.01 ? noteIntensity * 3.0 : 0;
    orb.material.emissiveIntensity = THREE.MathUtils.lerp(orb.material.emissiveIntensity, activeGlow, 0.15);

    // 2. MFCC (Timbral Envelope) -> Radial Radius Jitter
    const mfccDistortion = (mfcc[i] || 0) * 0.012;
    const angle = (i / 12) * Math.PI * 2 + Math.PI / 2;
    const currentRadius = this.baseRadius + mfccDistortion;
    
    const targetPos = new THREE.Vector3(
      Math.cos(angle) * currentRadius,
      Math.sin(angle) * currentRadius,
      0
    );
    orb.position.lerp(targetPos, 0.1);
  });

  // Base persistent rotation for the entire group
  this.group.rotation.z -= 0.002;

  // --- BLOOM EFFECTS ---
  bloom.strength = 0.4 + this.targetBloom;
  bloom.threshold = 0.2;
  bloom.radius = 0.8;

  // --- STATE PERSISTENCE ---
  this.prevRms = rms;
}

export function cleanup(scene) {
  // Standard cleanup for Shekere sketches
  Shekere.clearScene(scene);
}
