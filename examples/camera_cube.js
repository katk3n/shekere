/**
 * Camera + audio reactive example.
 *
 * Enable both Camera and Mic in the Control Panel before loading this sketch.
 * The camera VideoTexture is owned by Shekere and must not be disposed here.
 */
export function setup(scene) {
  this.geometry = new THREE.BoxGeometry(2.4, 2.4, 2.4);
  this.material = new THREE.MeshBasicMaterial({
    color: 0xffffff,
    side: THREE.DoubleSide
  });
  this.mesh = new THREE.Mesh(this.geometry, this.material);
  this.mesh.visible = false;
  scene.add(this.mesh);

  this.smoothBass = 0;
  this.smoothMid = 0;
  this.smoothHigh = 0;
  this.smoothVolume = 0;

  return {
    audio: { minFreqHz: 20, maxFreqHz: 8000 }
  };
}

export function update({ time, audio, camera, bloom, rgbShift, film, vignette }) {
  // Camera restart and device switching can replace the host-owned texture.
  if (this.material.map !== camera.texture) {
    this.material.map = camera.texture;
    this.material.needsUpdate = true;
  }

  this.mesh.visible = camera.active && camera.texture !== null;
  if (!this.mesh.visible) return;

  // Smooth the audio values to prevent abrupt visual movement.
  this.smoothBass = THREE.MathUtils.lerp(this.smoothBass, audio.bass, 0.12);
  this.smoothMid = THREE.MathUtils.lerp(this.smoothMid, audio.mid, 0.1);
  this.smoothHigh = THREE.MathUtils.lerp(this.smoothHigh, audio.high, 0.15);
  this.smoothVolume = THREE.MathUtils.lerp(this.smoothVolume, audio.volume, 0.08);

  // Bass makes the whole cube pulse, while high frequencies stretch it.
  const bassScale = 1 + this.smoothBass * 0.35;
  const highStretch = Math.sin(time * 10) * this.smoothHigh * 0.18;
  this.mesh.scale.set(
    bassScale + highStretch,
    bassScale - highStretch,
    bassScale
  );

  // Mid frequencies accelerate the rotation on both axes.
  const rotationSpeed = 0.25 + this.smoothMid * 1.8;
  this.mesh.rotation.x = time * rotationSpeed;
  this.mesh.rotation.y = time * rotationSpeed * 1.3;
  this.mesh.rotation.z = Math.sin(time * 3) * this.smoothHigh * 0.25;
  this.mesh.position.z = Math.sin(time * 2) * this.smoothMid * 0.25;

  // High frequencies and overall volume drive post-processing effects.
  bloom.strength = 0.2 + this.smoothBass * 1.8;
  bloom.radius = 0.35;
  bloom.threshold = 0.6;
  rgbShift.amount = this.smoothHigh * 0.012;
  film.intensity = this.smoothVolume * 0.45;
  vignette.offset = 1.1;
  vignette.darkness = 0.8 + this.smoothBass * 0.5;
}

export function cleanup(scene) {
  scene.remove(this.mesh);

  // Detach, but never dispose, the host-owned camera VideoTexture.
  this.material.map = null;
  this.geometry.dispose();
  this.material.dispose();
}
