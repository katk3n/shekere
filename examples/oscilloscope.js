/**
 * oscilloscope.js - Time-domain waveform example
 *
 * Draws a 1024-point oscilloscope trace from audio.waveform.mono. Replace
 * `mono` with `left` or `right` to inspect an individual stereo channel.
 */

const POINT_COUNT = 1024;
const TRACE_WIDTH = 12;
const TRACE_HEIGHT = 3;

export function setup(scene) {
  const positions = new Float32Array(POINT_COUNT * 3);
  for (let point = 0; point < POINT_COUNT; point++) {
    positions[point * 3] = (point / (POINT_COUNT - 1) - 0.5) * TRACE_WIDTH;
  }

  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
  const material = new THREE.LineBasicMaterial({ color: 0x34d399 });
  this.line = new THREE.Line(geometry, material);
  this.pointCount = POINT_COUNT;
  scene.add(this.line);

  const grid = new THREE.GridHelper(12, 12, 0x1f2937, 0x111827);
  grid.rotation.x = Math.PI / 2;
  scene.add(grid);
}

export function update({ audio }) {
  const waveform = audio.waveform.mono;
  const positions = this.line.geometry.attributes.position.array;
  const stride = waveform.length / this.pointCount;

  for (let point = 0; point < this.pointCount; point++) {
    positions[point * 3 + 1] = waveform[Math.floor(point * stride)] * TRACE_HEIGHT;
  }
  this.line.geometry.attributes.position.needsUpdate = true;
}

export function cleanup(scene) {
  Shekere.clearScene(scene);
}
