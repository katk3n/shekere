/**
 * Texture-state particles. Each texel stores x/y position, horizontal
 * velocity, and lifetime; the host advances that state before drawing.
 */

const STATE_SIZE = 32;
const PARTICLE_COUNT = STATE_SIZE * STATE_SIZE;
const CAMERA_FOV_DEGREES = 75;
const CAMERA_DISTANCE = 5;

function updateViewBounds(context) {
  const halfHeight = Math.tan(THREE.MathUtils.degToRad(CAMERA_FOV_DEGREES * 0.5)) * CAMERA_DISTANCE;
  const aspect = window.innerWidth / Math.max(1, window.innerHeight);
  context.viewHalfWidthNode.value = halfHeight * aspect;
  context.viewHalfHeightNode.value = halfHeight;
}

export function setup(scene) {
  this.state = Shekere.gpu.createFeedbackPass({
    name: "particle-state",
    width: STATE_SIZE,
    height: STATE_SIZE,
    format: "rgba16f",
    uniforms: { speed: 0.18, turbulence: 0.08 },
    build({ previous, uniforms, uv, deltaTime, time }) {
      const prior = previous.sample(uv);
      const random = (offset) => TSL.fract(
        TSL.sin(TSL.dot(uv.add(offset), TSL.vec2(12.9898, 78.233))).mul(43758.5453)
      );
      const startX = random(TSL.vec2(0, 0));
      const startY = random(TSL.vec2(19.19, 73.73));
      const velocitySeed = random(TSL.vec2(41.37, 11.91));
      const lifetimeSeed = random(TSL.vec2(7.17, 53.83));
      const expired = prior.a.lessThanEqual(0.001);
      const reset = TSL.vec4(
        startX,
        startY,
        velocitySeed.sub(0.5).mul(0.22),
        TSL.mix(0.55, 1, lifetimeSeed)
      );
      const verticalSpeed = uniforms.speed.mul(TSL.mix(0.55, 1.45, velocitySeed));
      const drift = TSL.sin(time.mul(1.7).add(startX.mul(6.283))).mul(uniforms.turbulence);
      const next = TSL.vec4(
        TSL.fract(prior.x.add(prior.z.add(drift).mul(deltaTime))),
        prior.y.add(verticalSpeed.mul(deltaTime)),
        prior.z,
        prior.a.sub(deltaTime.mul(TSL.mix(0.18, 0.42, lifetimeSeed)))
      );
      return TSL.select(expired.or(next.y.greaterThan(1)), reset, next);
    }
  });

  this.viewHalfWidthNode = TSL.uniform(1);
  this.viewHalfHeightNode = TSL.uniform(1);
  this.bassNode = TSL.uniform(0);
  this.midNode = TSL.uniform(0);
  this.highNode = TSL.uniform(0);
  this.volumeNode = TSL.uniform(0);
  this.timeNode = TSL.uniform(0);
  updateViewBounds(this);

  const geometry = new THREE.CircleGeometry(0.018, 6);
  const material = new THREE.MeshBasicNodeMaterial({ transparent: true });
  const index = TSL.float(TSL.instanceIndex);
  const stateUv = TSL.vec2(
    TSL.mod(index, STATE_SIZE).add(0.5).div(STATE_SIZE),
    TSL.floor(index.div(STATE_SIZE)).add(0.5).div(STATE_SIZE)
  );
  const particle = this.state.node.sample(stateUv);
  const particleSeed = TSL.fract(TSL.sin(TSL.dot(stateUv, TSL.vec2(29.17, 61.43))).mul(31821.371));
  const pulse = TSL.sin(this.timeNode.mul(5).add(particleSeed.mul(31.416))).mul(0.5).add(0.5);
  const particleSize = TSL.mix(0.7, 1.45, particleSeed)
    .mul(TSL.float(1).add(this.bassNode.mul(TSL.mix(0.6, 1.8, pulse))));
  const horizontalPosition = particle.x.mul(2).sub(1).mul(this.viewHalfWidthNode).mul(0.98);
  const verticalPosition = particle.y.mul(2).sub(1).mul(this.viewHalfHeightNode).mul(0.98);
  material.positionNode = TSL.positionLocal.mul(particleSize).add(TSL.vec3(
    horizontalPosition,
    verticalPosition,
    0
  ));
  const colorPhase = particleSeed.mul(6.283)
    .add(this.timeNode.mul(this.midNode.mul(1.4).add(0.12)))
    .add(this.midNode.mul(3.142));
  const palette = TSL.vec3(
    TSL.cos(colorPhase).mul(0.5).add(0.5),
    TSL.cos(colorPhase.add(2.094)).mul(0.5).add(0.5),
    TSL.cos(colorPhase.add(4.189)).mul(0.5).add(0.5)
  );
  const contrastedPalette = TSL.mix(palette, palette.mul(palette).mul(1.6), this.highNode);
  const sparkle = TSL.sin(this.timeNode.mul(9).add(particleSeed.mul(62.832)))
    .mul(0.5)
    .add(0.5);
  const sparkleIntensity = TSL.mix(1, TSL.mix(0.35, 1.65, sparkle), this.highNode);
  const emission = TSL.float(1)
    .add(this.volumeNode.mul(1.8))
    .add(this.bassNode.mul(3.2));
  material.colorNode = contrastedPalette.mul(sparkleIntensity).mul(emission);
  material.opacityNode = TSL.smoothstep(0, 0.18, particle.a)
    .mul(TSL.mix(0.7, 1, sparkleIntensity));

  this.particles = new THREE.InstancedMesh(geometry, material, PARTICLE_COUNT);
  this.geometry = geometry;
  this.material = material;
  scene.add(this.particles);
}

export function update({ time, audio, bloom }) {
  updateViewBounds(this);
  this.timeNode.value = time;
  this.bassNode.value = audio.bass;
  this.midNode.value = audio.mid;
  this.highNode.value = audio.high;
  this.volumeNode.value = audio.volume;

  this.state.update({
    uniforms: {
      speed: 0.12 + audio.bass * 0.5,
      turbulence: 0.04 + audio.high * 0.2
    }
  });

  bloom.strength = 0.65 + audio.volume * 1.5 + audio.bass * 3;
  bloom.radius = 0.45 + audio.mid * 0.3;
  bloom.threshold = 0.85 - audio.high * 0.3;
}

export function cleanup(scene) {
  scene.remove(this.particles);
  this.geometry.dispose();
  this.material.dispose();
  this.state.dispose();
}
