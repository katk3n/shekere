// spectrum.js — オーディオスペクトラムビジュアライザー
// 256バンドの FFT データを使い、周波数ごとに高さが変わる棒グラフを描画します。
//
// context.audio.bands: number[256]  各バンドの強度 (0.0~1.0, 27.5Hz~4186Hz をカバー)
// context.audio.bass:  number       27.5~250 Hz 平均 (ピアノ低音域)
// context.audio.mid:   number       250Hz~2kHz 平均  (ピアノ中音域)
// context.audio.high:  number       2kHz~4186Hz 平均 (ピアノ高音域)
// context.audio.volume: number      全体音量   (0.0 ~ 1.0)

const BAND_COUNT = 256;
const MAX_HEIGHT = 8.0;
// カメラ(z=5, FOV=75°)の可視幅 ≈ 2 × tan(37.5°) × 5 ≈ 7.7 ユニット
// 256バンドが収まるよう各スロット幅を逆算する
const VISIBLE_WIDTH = 7.2;
const BAR_SLOT   = VISIBLE_WIDTH / BAND_COUNT;  // ≈ 0.028 /バンド
const BAR_WIDTH  = BAR_SLOT * 0.7;              // ≈ 0.020
const BAR_SPACING = BAR_SLOT * 0.3;             // ≈ 0.008
const TOTAL_WIDTH = BAND_COUNT * BAR_SLOT;      //   7.2

export function setup(scene) {
  // カメラがデフォルト位置(z=5)なので、スペクトラム全体が入るよう調整
  // ユーザーがカメラを持っていないため scene に直接メタデータを補足する
  this.bars = [];

  const geometry = new THREE.BoxGeometry(BAR_WIDTH, 1, BAR_WIDTH);

  for (let i = 0; i < BAND_COUNT; i++) {
    // 低音が左、高音が右に並ぶ
    const x = (i * (BAR_WIDTH + BAR_SPACING)) - TOTAL_WIDTH / 2 + BAR_WIDTH / 2;

    // 周波数に応じてグラデーションカラー（低音=青、中音=緑、高音=赤）
    const t = i / (BAND_COUNT - 1);
    const color = new THREE.Color().setHSL(0.66 - t * 0.66, 1.0, 0.55);

    const material = new THREE.MeshStandardMaterial({ color, roughness: 0.4, metalness: 0.3 });
    const bar = new THREE.Mesh(geometry, material);

    bar.position.set(x, 0, 0);
    bar.scale.set(1, 0.01, 1); // 初期状態は高さほぼゼロ
    scene.add(bar);
    this.bars.push(bar);
  }

  // ライト
  this.ambientLight = new THREE.AmbientLight(0xffffff, 0.4);
  scene.add(this.ambientLight);

  this.pointLight = new THREE.PointLight(0xffffff, 60, 50);
  this.pointLight.position.set(0, 10, 10);
  scene.add(this.pointLight);
}

export function update({ time, audio }) {
  const { bands, volume } = audio;

  for (let i = 0; i < BAND_COUNT; i++) {
    const bar = this.bars[i];
    const targetHeight = Math.max(0.01, bands[i] * MAX_HEIGHT);

    // 滑らかに追従（lerp）
    bar.scale.y += (targetHeight - bar.scale.y) * 0.25;

    // バーの底辺を揃えるため、スケールに合わせてY座標を補正
    bar.position.y = bar.scale.y / 2;

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
    bar.geometry.dispose();
    bar.material.dispose();
  }
  scene.remove(this.ambientLight);
  scene.remove(this.pointLight);
}
