# スケッチの書き方

Shekereでは、JavaScriptと **Three.js** を使ってビジュアルを作成できます。ShekereはThree.jsのラッパーとして機能し、レンダリングループやシーン、カメラの管理を自動的に行うため、ユーザーはビジュアルのロジックに集中できます。

## ライフサイクルAPI

標準的なThree.jsのループを書く代わりに、Shekereのスケッチでは特定の関数をエクスポートします。

### 1. `setup(scene)`
スケッチが読み込まれたときに一度だけ呼び出されます。3Dオブジェクト、ライト、マテリアルの初期化に使用します。
- **引数**: `scene` (`THREE.Scene` オブジェクト)。
- **戻り値**: オーディオ範囲やレンダラーのプロパティを設定するためのオプションの構成オブジェクト。

```javascript
export function setup(scene) {
  const geometry = new THREE.BoxGeometry(1, 1, 1);
  const material = new THREE.MeshBasicMaterial({ color: 0x00ff00 });
  this.cube = new THREE.Mesh(geometry, material);
  scene.add(this.cube);

  return {
    audio: { minFreqHz: 80, maxFreqHz: 2000 }
  };
}
```

### 2. `update(context)`
毎フレーム（毎秒約60回）呼び出されます。シーンのアニメーションに使用します。
- **引数**: `context` (リアルタイムデータを含むオブジェクト)。

```javascript
export function update({ time, audio, bloom }) {
  // 時間の経過に合わせて立方体を回転させる
  this.cube.rotation.x = time;
  
  // ブルーム（発光）の強度を音量に反応させる
  bloom.strength = audio.volume * 2.0;
}
```

### 3. `cleanup(scene)`
スケッチが切り替わる直前、またはリロードされる直前に呼び出されます。
- **重要**: メモリリークを防ぐために、オブジェクトをクリーンアップする必要があります。後述のヘルパー関数を使用するのが最も簡単です。

```javascript
export function cleanup(scene) {
  Shekere.clearScene(scene);
}
```

## `context` オブジェクト

`update` 関数は、以下のデータを含むリッチなオブジェクトを受け取ります：

| プロパティ | 型 | 説明 |
| :--- | :--- | :--- |
| `time` | `number` | 起動からの経過時間（秒）。 |
| `audio` | `object` | 処理済みのオーディオデータ（volume, bass, mid, high, bands）。 |
| `midi` | `object` | MIDI入力データ (`midi.notes[0-127]`, `midi.cc[0-127]`)。 |
| `osc` | `object` | アドレスごとの最新のOSCデータ（例：`osc['/play']`）。 |
| `bloom` | `object` | ポストプロセスのブルーム制御（`strength`, `radius`, `threshold`）。 |
| `rgbShift` | `object` | RGBシフトの量を制御。 |
| `film` | `object` | フィルムグレインの制御（`intensity`）。 |
| `vignette` | `object` | ヴィニエットの制御（`offset`, `darkness`）。 |

## オーディオデータの詳細

### 基本プロパティ (`audio`)
- `audio.volume`: 全体的な音量 (0.0 - 1.0)。
- `audio.bass` / `mid` / `high`: 特定の周波数範囲の平均強度。
- `audio.bands`: 256個の周波数ビン（FFTデータ）の配列。

### 高度な特徴量 (`audio.features`)
Shekereは、深い音声解析のために **Meyda.js** を使用しています。
- `audio.features.rms`: 知覚的な音量。
- `audio.features.zcr`: ゼロ交差率（パーカッシブな音の検出に有効）。
- `audio.features.spectralCentroid`: 音の「明るさ」を示します。

## `Shekere` グローバルオブジェクト

開発を支援するためのグローバルユーティリティが提供されています：

- `Shekere.clearScene(container)`: シーン内のすべてのオブジェクトとマテリアルを安全に破棄します。
- `Shekere.SKETCH_DIR`: 現在のスケッチが保存されているディレクトリの絶対パス。テクスチャなどのローカルアセットを読み込む際に便利です。
- `THREE`: Three.jsライブラリ全体がグローバルに利用可能です。インポートは不要です。
- `TSL`: Three.js Shading Language がグローバルに利用可能です。シェーダーノードの構築に使用します。

## ポストプロセス (Post-Processing)

コードから直接ビジュアルエフェクトを制御できます。変更はコントロールパネルのUIと自動的に同期されます。

```javascript
export function update({ audio, bloom, rgbShift }) {
  bloom.strength = audio.bass * 3.0;
  rgbShift.amount = audio.high * 0.02;
}
```
