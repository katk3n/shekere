/**
 * spectrum.js — 256-band Audio Spectrum Visualizer
 *
 * Renders a full-frequency bar chart using FFT data from the microphone.
 * Bars grow upward from the bottom of the screen; color shifts from blue
 * (low frequencies, left) to red (high frequencies, right).
 *
 * Frequency range: 27.5 Hz (piano A0) to 4,186 Hz (piano C8)
 * Divided into 256 linear bands — one bar per band.
 *
 * Audio data used:
 *   context.audio.bands[256]  per-band intensity  (0.0 – 1.0)
 *   context.audio.bass        27.5 – 250 Hz avg   (0.0 – 1.0)
 *   context.audio.mid         250 Hz – 2 kHz avg  (0.0 – 1.0)
 *   context.audio.high        2 – 4.2 kHz avg     (0.0 – 1.0)
 *   context.audio.volume      overall loudness    (0.0 – 1.0)
 *
 * Camera assumptions: PerspectiveCamera at z=5, FOV=75°.
 * Adjust VISIBLE_WIDTH and BOTTOM_Y if you change the camera.
 */

const BAND_COUNT = 256;
const MAX_HEIGHT = 7.5; // 画面のほぼ高さ全体
// 画面下端の座標 (z=5, FOV=75° の場合 約 -3.8)
const BOTTOM_Y = -3.8;
// 画面の横幅いっぱいに広げる (アスペクト比を考慮して大きめに設定)
const VISIBLE_WIDTH = 13.0; 
const BAR_SLOT   = VISIBLE_WIDTH / BAND_COUNT;
const BAR_WIDTH  = BAR_SLOT * 0.8;
const BAR_SPACING = BAR_SLOT * 0.2;
const TOTAL_WIDTH = BAND_COUNT * BAR_SLOT;

export function setup(scene) {
  this.bars = [];
  this.geometry = new THREE.BoxGeometry(BAR_WIDTH, 1, BAR_WIDTH);

  for (let i = 0; i < BAND_COUNT; i++) {
    const x = i * BAR_SLOT - TOTAL_WIDTH / 2 + BAR_WIDTH / 2;

    // 周波数に応じてグラデーションカラー（低音=青、中音=緑、高音=赤）
    const t = i / (BAND_COUNT - 1);
    const color = new THREE.Color().setHSL(0.66 - t * 0.66, 1.0, 0.55);

    const material = new THREE.MeshStandardMaterial({ color, roughness: 0.4, metalness: 0.3 });
    const bar = new THREE.Mesh(this.geometry, material);

    bar.position.set(x, BOTTOM_Y, 0);
    bar.scale.set(1, 0.01, 1);
    scene.add(bar);
    this.bars.push(bar);
  }

  // ライト
  this.ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
  scene.add(this.ambientLight);

  this.pointLight = new THREE.PointLight(0xffffff, 60, 50);
  this.pointLight.position.set(0, 5, 5); // ライトの位置も少し調整
  scene.add(this.pointLight);
}

export function update({ time, audio }) {
  const { bands, volume } = audio;

  for (let i = 0; i < BAND_COUNT; i++) {
    const bar = this.bars[i];
    const targetHeight = Math.max(0.01, bands[i] * MAX_HEIGHT);

    // 滑らかに追従（lerp）
    bar.scale.y += (targetHeight - bar.scale.y) * 0.25;

    // バーの底辺を BOTTOM_Y に揃えるため、Y座標を補正
    bar.position.y = BOTTOM_Y + bar.scale.y / 2;

    // 音量に合わせてほんのり明るさが変わる
    bar.material.emissiveIntensity = volume * 0.5;
    bar.material.emissive = bar.material.color;
  }

  // ポイントライトを時間でゆっくり動かす
  this.pointLight.position.x = Math.sin(time * 0.3) * 15;
}

export function cleanup(scene) {
  for (const bar of this.bars) {
    scene.remove(bar);
    bar.material.dispose(); // material は各バーで独立しているので個別に破棄
  }
  this.geometry.dispose(); // geometry は共有なので1回だけ破棄
  scene.remove(this.ambientLight);
  scene.remove(this.pointLight);
}
