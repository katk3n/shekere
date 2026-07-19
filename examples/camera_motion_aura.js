/**
 * GPU camera motion trail with audio-reactive color and bloom.
 *
 * Enable Camera and Mic in the Control Panel before loading this sketch.
 * Camera and motion textures belong to Shekere and must not be disposed here.
 */

export function setup(scene) {
  this.auraColorNode = TSL.uniform(new THREE.Color(0x35a7ff));
  this.auraIntensityNode = TSL.uniform(1.5);

  const cameraColor = Shekere.camera.textureNode.sample(TSL.uv()).rgb;
  const trail = Shekere.camera.motion.trailNode.sample(TSL.uv()).r;
  // Fade smoothly, then clamp weak trail values to zero so bloom cannot keep
  // amplifying a faint afterimage indefinitely.
  const visibleTrail = trail.mul(TSL.smoothstep(0.02, 0.08, trail));
  const aura = this.auraColorNode.mul(visibleTrail).mul(this.auraIntensityNode);
  this.material = new THREE.MeshBasicNodeMaterial({
    depthTest: false,
    depthWrite: false,
    side: THREE.DoubleSide
  });
  this.material.vertexNode = TSL.vec4(TSL.positionGeometry.xy, 0, 1);
  this.material.colorNode = cameraColor.add(aura);

  this.geometry = new THREE.PlaneGeometry(2, 2);
  this.screen = new THREE.Mesh(this.geometry, this.material);
  this.screen.frustumCulled = false;
  scene.add(this.screen);

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

export function update({ time, audio, camera, bloom }) {
  this.screen.visible = camera.active;

  this.smoothBass = THREE.MathUtils.lerp(this.smoothBass, audio.bass, 0.12);
  this.smoothHigh = THREE.MathUtils.lerp(this.smoothHigh, audio.high, 0.14);
  const hue = (0.55 + time * 0.025 + this.smoothHigh * 0.25) % 1;
  this.auraColor.setHSL(hue, 0.95, 0.58);
  this.auraColorNode.value.copy(this.auraColor);
  this.auraIntensityNode.value = 1.5 + this.smoothBass * 5;

  bloom.strength = 0.8 + this.smoothBass * 3.5;
  bloom.radius = 0.65;
  // Keep ordinary camera pixels out of bloom; only the HDR aura exceeds 1.0.
  bloom.threshold = 1.1;
}

export function cleanup(scene) {
  Shekere.clearScene(scene);
}
