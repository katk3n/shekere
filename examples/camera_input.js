/**
 * Camera background + audio-reactive mono waveform.
 *
 * Enable both Camera and Mic in the Control Panel before loading this sketch.
 * The camera VideoTexture belongs to Shekere and must not be disposed here.
 */

const WAVEFORM_POINTS = 512;

export function setup(scene) {
  this.scene = scene;
  this.previousBackground = scene.background;
  this.cameraTexture = null;
  this.smoothedSamples = new Float32Array(WAVEFORM_POINTS);
  this.waveformColor = new THREE.Color();

  const positions = new Float32Array(WAVEFORM_POINTS * 3);
  for (let index = 0; index < WAVEFORM_POINTS; index++) {
    const x = THREE.MathUtils.mapLinear(index, 0, WAVEFORM_POINTS - 1, -6, 6);
    positions[index * 3] = x;
  }

  this.waveformGeometry = new THREE.BufferGeometry();
  this.positionAttribute = new THREE.BufferAttribute(positions, 3);
  this.positionAttribute.setUsage(THREE.DynamicDrawUsage);
  this.waveformGeometry.setAttribute("position", this.positionAttribute);

  const materialOptions = {
    transparent: true,
    blending: THREE.AdditiveBlending,
    depthTest: false,
    depthWrite: false
  };

  this.outerMaterial = new THREE.LineBasicMaterial({
    ...materialOptions,
    color: 0x3366ff,
    opacity: 0.12
  });
  this.glowMaterial = new THREE.LineBasicMaterial({
    ...materialOptions,
    color: 0x66aaff,
    opacity: 0.35
  });
  this.coreMaterial = new THREE.LineBasicMaterial({
    ...materialOptions,
    color: 0xffffff,
    opacity: 0.95
  });

  this.outerLine = new THREE.Line(this.waveformGeometry, this.outerMaterial);
  this.glowLine = new THREE.Line(this.waveformGeometry, this.glowMaterial);
  this.coreLine = new THREE.Line(this.waveformGeometry, this.coreMaterial);

  this.outerLine.frustumCulled = false;
  this.glowLine.frustumCulled = false;
  this.coreLine.frustumCulled = false;
  this.outerLine.scale.y = 1.12;
  this.glowLine.scale.y = 1.05;
  this.outerLine.renderOrder = 1;
  this.glowLine.renderOrder = 2;
  this.coreLine.renderOrder = 3;

  scene.add(this.outerLine, this.glowLine, this.coreLine);

  this.smoothBass = 0;
  this.smoothHigh = 0;
  this.smoothVolume = 0;

  return {
    audio: { minFreqHz: 20, maxFreqHz: 8000 }
  };
}

export function update({ time, audio, camera, bloom, rgbShift, film }) {
  // Use the host-owned camera texture as the full-render background.
  if (this.cameraTexture !== camera.texture) {
    this.cameraTexture = camera.texture;
  }
  const nextBackground = camera.active ? this.cameraTexture : null;
  if (this.scene.background !== nextBackground) {
    this.scene.background = nextBackground;
  }

  this.smoothBass = THREE.MathUtils.lerp(this.smoothBass, audio.bass, 0.12);
  this.smoothHigh = THREE.MathUtils.lerp(this.smoothHigh, audio.high, 0.14);
  this.smoothVolume = THREE.MathUtils.lerp(this.smoothVolume, audio.volume, 0.1);

  const waveform = audio.waveform?.mono;
  const amplitude = 1.2 + this.smoothBass * 2.8 + this.smoothVolume * 0.8;
  const smoothing = 0.28 + this.smoothHigh * 0.35;

  for (let index = 0; index < WAVEFORM_POINTS; index++) {
    const sourceIndex = waveform?.length
      ? Math.floor(index * (waveform.length - 1) / (WAVEFORM_POINTS - 1))
      : 0;
    const sample = waveform?.length
      ? THREE.MathUtils.clamp(waveform[sourceIndex], -1, 1)
      : 0;

    this.smoothedSamples[index] = THREE.MathUtils.lerp(
      this.smoothedSamples[index],
      sample,
      smoothing
    );

    const y = this.smoothedSamples[index] * amplitude;
    this.positionAttribute.setY(index, y);
  }
  this.positionAttribute.needsUpdate = true;

  // Slowly shift the neon hue while high frequencies add faster variation.
  const hue = (time * 0.025 + this.smoothHigh * 0.3) % 1;
  this.waveformColor.setHSL(hue, 0.9, 0.55);
  this.outerMaterial.color.copy(this.waveformColor).multiplyScalar(1.4);
  this.glowMaterial.color.copy(this.waveformColor).multiplyScalar(2.5);
  this.coreMaterial.color.copy(this.waveformColor).multiplyScalar(4);

  this.outerMaterial.opacity = 0.08 + this.smoothVolume * 0.18;
  this.glowMaterial.opacity = 0.2 + this.smoothVolume * 0.35;
  this.coreMaterial.opacity = 0.65 + this.smoothVolume * 0.35;

  // Only the HDR waveform exceeds this threshold; camera pixels remain natural.
  bloom.strength = 0.45 + this.smoothBass * 2.2;
  bloom.radius = 0.4;
  bloom.threshold = 1.1;
  rgbShift.amount = this.smoothHigh * 0.003;
  film.intensity = this.smoothVolume * 0.08;
}

export function cleanup(scene) {
  scene.background = this.previousBackground;
  // Release the reference without disposing the host-owned VideoTexture.
  this.cameraTexture = null;
  this.scene = null;
  Shekere.clearScene(scene);
}
