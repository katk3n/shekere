/**
 * Persistent GPU ripple seeded by ADR 0008 camera motion.
 * Enable Camera and Mic before loading this sketch.
 */

const ATTACK_COOLDOWN_SECONDS = 0.15;
const ATTACK_MIN_FLUX = 0.0015;
const ATTACK_THRESHOLD_MULTIPLIER = 2.5;

export function setup(scene) {
  this.scene = scene;
  this.previousBackgroundNode = scene.backgroundNode;
  this.ripple = null;
  this.rippleWidth = 0;
  this.rippleHeight = 0;
  this.bassNode = TSL.uniform(0);
  this.midNode = TSL.uniform(0);
  this.highNode = TSL.uniform(0);
  this.volumeNode = TSL.uniform(0);
  this.glowNode = TSL.uniform(0);
  this.timeNode = TSL.uniform(0);
  this.glowLevel = 0;
  this.previousTime = null;
  this.previousBands = null;
  this.fluxBaseline = 0;
  this.lastAttackTime = -Infinity;

  return { camera: { motion: { enabled: true, threshold: 0.15 } } };
}

export function update({ time, camera, audio, bloom }) {
  const elapsed = this.previousTime === null
    ? 0
    : Math.min(Math.max(time - this.previousTime, 0), 0.1);
  this.previousTime = time;
  const audioPeak = Math.max(audio.volume, audio.bass, audio.mid * 0.65, audio.high * 0.4);
  this.glowLevel = Math.max(audioPeak, this.glowLevel * Math.exp(-elapsed * 0.45));

  const bands = audio.bands;
  let spectralFlux = 0;
  if (!this.previousBands || this.previousBands.length !== bands.length) {
    this.previousBands = new Float32Array(bands.length);
  } else if (bands.length > 0) {
    for (let index = 0; index < bands.length; index++) {
      spectralFlux += Math.max(bands[index] - this.previousBands[index], 0);
    }
    spectralFlux /= bands.length;
  }
  this.previousBands.set(bands);

  const attackThreshold = Math.max(
    ATTACK_MIN_FLUX,
    this.fluxBaseline * ATTACK_THRESHOLD_MULTIPLIER
  );
  const attackDetected = spectralFlux > attackThreshold
    && time - this.lastAttackTime >= ATTACK_COOLDOWN_SECONDS;
  if (attackDetected) this.lastAttackTime = time;

  const baselineInput = attackDetected
    ? Math.min(spectralFlux, attackThreshold)
    : spectralFlux;
  const baselineAlpha = 1 - Math.exp(-elapsed * 4);
  this.fluxBaseline += (baselineInput - this.fluxBaseline) * baselineAlpha;

  this.timeNode.value = time;
  this.bassNode.value = audio.bass;
  this.midNode.value = audio.mid;
  this.highNode.value = audio.high;
  this.volumeNode.value = audio.volume;
  this.glowNode.value = this.glowLevel;

  if (!camera.motion.active && !this.ripple) return;
  const width = camera.motion.width;
  const height = camera.motion.height;

  if (camera.motion.active
    && (!this.ripple || width !== this.rippleWidth || height !== this.rippleHeight)) {
    this.ripple?.dispose();
    this.rippleWidth = width;
    this.rippleHeight = height;
    this.ripple = Shekere.gpu.createFeedbackPass({
      name: "motion-ripple",
      width,
      height,
      format: "rgba16f",
      textures: ["motion"],
      uniforms: { decay: 0.97, intensity: 1, attack: 0 },
      build({ previous, textures, uniforms, uv }) {
        const texel = TSL.vec2(1 / width, 1 / height);
        const state = previous.sample(uv);
        const current = state.r;
        const older = state.g;
        const left = previous.sample(uv.sub(TSL.vec2(texel.x, 0))).r;
        const right = previous.sample(uv.add(TSL.vec2(texel.x, 0))).r;
        const down = previous.sample(uv.sub(TSL.vec2(0, texel.y))).r;
        const up = previous.sample(uv.add(TSL.vec2(0, texel.y))).r;
        const downLeft = previous.sample(uv.sub(texel)).r;
        const upRight = previous.sample(uv.add(texel)).r;
        const upLeft = previous.sample(
          uv.add(TSL.vec2(texel.x.negate(), texel.y))
        ).r;
        const downRight = previous.sample(
          uv.add(TSL.vec2(texel.x, texel.y.negate()))
        ).r;

        // A nine-point Laplacian avoids the diamond-shaped propagation caused
        // by taking the maximum of horizontal and vertical neighbours.
        const cardinal = left.add(right).add(down).add(up).mul(4);
        const diagonal = downLeft.add(upRight).add(upLeft).add(downRight);
        const laplacian = cardinal.add(diagonal).sub(current.mul(20)).div(6);
        const wave = current.mul(2)
          .sub(older)
          .add(laplacian.mul(0.28))
          .mul(uniforms.decay);
        const seed = textures.motion.sample(uv).r
          .mul(uniforms.intensity)
          .mul(uniforms.attack)
          .mul(0.2);
        const next = TSL.clamp(wave.add(seed), -4, 4);

        // R is the current wave height and G is the preceding height.
        return TSL.vec4(next, current, 0, 1);
      }
    });

    const screenUv = TSL.screenUV.flipY();
    const cameraColor = Shekere.camera.textureNode.sample(screenUv).rgb;
    const ripple = TSL.abs(this.ripple.node.sample(screenUv).r);
    const rippleLevel = TSL.clamp(ripple, 0, 1);
    const visibleRipple = rippleLevel.mul(TSL.smoothstep(0.015, 0.08, rippleLevel));
    const colorPhase = this.timeNode.mul(this.midNode.mul(0.35).add(0.05))
      .add(this.midNode.mul(3.142))
      .add(this.bassNode.mul(0.7));
    const palette = TSL.vec3(
      TSL.cos(colorPhase).mul(0.5).add(0.5),
      TSL.cos(colorPhase.add(2.094)).mul(0.5).add(0.5),
      TSL.cos(colorPhase.add(4.189)).mul(0.5).add(0.5)
    );
    const contrastedPalette = TSL.mix(palette, palette.mul(palette).mul(1.7), this.highNode);
    const shimmer = TSL.sin(
      screenUv.x.mul(73)
        .add(screenUv.y.mul(47))
        .add(this.timeNode.mul(11))
    ).mul(0.5).add(0.5);
    const shimmerIntensity = TSL.mix(1, TSL.mix(0.55, 1.55, shimmer), this.highNode);
    const emission = TSL.float(1.2)
      .add(this.glowNode.mul(2))
      .add(visibleRipple.mul(1.75))
      .add(this.volumeNode.mul(0.2));
    const aura = contrastedPalette
      .mul(visibleRipple)
      .mul(shimmerIntensity)
      .mul(emission);
    this.scene.backgroundNode = cameraColor.add(aura);
  }

  this.ripple.update({
    textures: { motion: Shekere.camera.motion.maskNode },
    uniforms: {
      decay: 0.975 + audio.mid * 0.02,
      intensity: 1 + audio.bass * 3,
      attack: attackDetected ? 1 : 0
    }
  });

  bloom.strength = 0.6 + this.glowLevel + audio.volume * 0.2;
  bloom.radius = 0.45 + this.glowLevel * 0.1 + audio.mid * 0.2;
  bloom.threshold = 1.05 - this.glowLevel * 0.08 - audio.high * 0.05;
}

export function cleanup(scene) {
  scene.backgroundNode = this.previousBackgroundNode;
  this.ripple?.dispose();
}
