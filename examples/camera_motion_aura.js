/**
 * GPU camera motion trail with audio-reactive color and bloom.
 *
 * Enable Camera and Mic in the Control Panel before loading this sketch.
 * Camera and motion textures belong to Shekere and must not be disposed here.
 */

export function setup(scene) {
  this.previousBackgroundNode = scene.backgroundNode;
  this.auraColorNode = TSL.uniform(new THREE.Color(0x35a7ff));
  this.auraIntensityNode = TSL.uniform(1.5);

  const screenUv = TSL.screenUV.flipY();
  const cameraColor = Shekere.camera.textureNode.sample(screenUv).rgb;
  const trail = Shekere.camera.motion.trailNode.sample(screenUv).r;
  // Fade smoothly, then clamp weak trail values to zero so bloom cannot keep
  // amplifying a faint afterimage indefinitely.
  const visibleTrail = trail.mul(TSL.smoothstep(0.02, 0.08, trail));
  const aura = this.auraColorNode.mul(visibleTrail).mul(this.auraIntensityNode);
  scene.backgroundNode = cameraColor.add(aura);

  this.smoothBass = 0;
  this.smoothHigh = 0;
  this.auraColor = new THREE.Color();

  return {
    audio: { minFreqHz: 20, maxFreqHz: 8000 },
    camera: {
      motion: {
        enabled: true,
        threshold: 0.08,
        blur: 6,
        decay: 0.94
      }
    }
  };
}

export function update({ time, audio, bloom }) {
  this.smoothBass = THREE.MathUtils.lerp(this.smoothBass, audio.bass, 0.12);
  this.smoothHigh = THREE.MathUtils.lerp(this.smoothHigh, audio.high, 0.14);
  const hue = (0.55 + time * 0.025 + this.smoothHigh) % 1;
  this.auraColor.setHSL(hue, 0.95, 0.58);
  this.auraColorNode.value.copy(this.auraColor);
  this.auraIntensityNode.value = 1.5 + this.smoothBass * 5;

  bloom.strength = 0.8 + this.smoothBass * 3.5;
  bloom.radius = 0.65;
  // Keep ordinary camera pixels out of bloom; only the HDR aura exceeds 1.0.
  bloom.threshold = 1.1;
}

export function cleanup(scene) {
  scene.backgroundNode = this.previousBackgroundNode;
}
