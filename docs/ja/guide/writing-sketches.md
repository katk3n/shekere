# スケッチの書き方

Shekereでは、JavaScriptと **Three.js** を使ってビジュアルを作成できます。ShekereはThree.jsのラッパーとして機能し、レンダリングループやシーン、カメラの管理を自動的に行うため、ユーザーはビジュアルのロジックに集中できます。

## ライフサイクルAPI

標準的なThree.jsのループを書く代わりに、Shekereのスケッチでは特定の関数をエクスポートします。

### 1. `setup(scene)`
スケッチが読み込まれたときに一度だけ呼び出されます。3Dオブジェクト、ライト、マテリアルの初期化に使用します。
- **引数**: `scene` (`THREE.Scene` オブジェクト)。
- **戻り値**: オーディオ範囲、レンダラーのプロパティ、またはカメラモーション解析を設定するためのオプションの構成オブジェクト。

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

`Shekere.clearScene(scene)`はすべての子孫objectを削除し、Mesh、Line、Points、
Spriteで使用されるgeometryとmaterialを重複なく一度だけdisposeします。
所有権を判断できないtextureやscene objectに直接属さないリソースはdisposeしません。
スケッチ所有のtexture、event listener、その他の外部リソースは`cleanup`内で明示的に
解放してください。

## `context` オブジェクト

`update` 関数は、以下のデータを含むリッチなオブジェクトを受け取ります：

| プロパティ | 型 | 説明 |
| :--- | :--- | :--- |
| `time` | `number` | 起動からの経過時間（秒）。 |
| `camera` | `object` | ライブカメラの状態、ホスト所有の`VideoTexture`、任意の[モーションテクスチャ](./camera-motion.md)。 |
| `audio` | `object` | 処理済みのオーディオデータ（volume、bands、features、waveform）。 |
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
- `audio.waveform.mono`、`.left`、`.right`: それぞれ4096個の正規化済み
  サンプルを持つ、再利用可能な `Float32Array` の時間領域バッファです。
  モノラル入力では left/right も同一になり、キャプチャ停止中はゼロで満たされます。

### 高度な特徴量 (`audio.features`)
Shekereは、深い音声解析のために **Meyda.js** を使用しています。
- `audio.features.rms`: 知覚的な音量。
- `audio.features.zcr`: ゼロ交差率（パーカッシブな音の検出に有効）。
- `audio.features.spectralCentroid`: 音の「明るさ」を示します。

## `Shekere` グローバルオブジェクト

開発を支援するためのグローバルユーティリティが提供されています：

- `Shekere.clearScene(container)`: 子孫objectを削除し、textureには触れず、sceneの
  geometryとmaterialを重複なくdisposeします。
- `Shekere.SKETCH_DIR`: 現在のスケッチが保存されているディレクトリの絶対パス。テクスチャなどのローカルアセットを読み込む際に便利です。
- `Shekere.camera.textureNode`: ライブカメラ映像を利用するための、同一性が安定した
  ホスト所有TSL nodeです。停止中は黒いfallbackを参照します。
- `Shekere.camera.motion.maskNode` / `trailNode`: `setup(scene)`でカメラモーションの
  TSL graphを構築するための、同一性が安定したホスト所有nodeです。
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
