# スケッチの書き方

Shekereでは、JavaScriptを使ってビジュアルを作成できます。このガイドでは、主要なAPIとビジュアライザーとのやり取りについて説明します。

## スケッチAPI

Shekereのすべてのスケッチは、`update` 関数をエクスポートする必要があります。この関数は、ビジュアルを描画するために毎フレーム（通常は毎秒60回）呼び出されます。

```javascript
/**
 * @param {CanvasRenderingContext2D} ctx - 2D描画コンテキスト
 * @param {number} width - ビジュアライザーウィンドウの現在の幅
 * @param {number} height - ビジュアライザーウィンドウの現在の高さ
 * @param {object} audio - Meydaによって提供される音声解析データ
 */
export function update(ctx, width, height, audio) {
  // ここに描画ロジックを書きます
}
```

### 1. コンテキスト (`ctx`)
Shekereは標準的な `CanvasRenderingContext2D` を提供します。`fillRect()`、`stroke()`、`beginPath()` など、すべての標準的なCanvas APIメソッドを使用できます。

### 2. サイズ (`width` と `height`)
これらの値は、ビジュアライザーウィンドウのサイズを表します。Shekereはウィンドウのリサイズを自動的に処理するため、要素の配置には常にこれらの変数を使用することをお勧めします。

### 3. オーディオデータ (`audio`)
`audio` オブジェクトには、[Meyda](https://meyda.js.org/) ライブラリを使用して抽出されたリアルタイム解析データが含まれています。主なプロパティは以下の通りです。

| プロパティ | 型 | 説明 |
| :--- | :--- | :--- |
| `rms` | `number` | ルート平均二乗（知覚的な音量）。おおよそ 0.0 から 1.0 の範囲です。 |
| `energy` | `number` | 音声信号の総エネルギー。 |
| `zcr` | `number` | ゼロ交差率（ノイズと音色の判別に役立ちます）。 |
| `amplitudeSpectrum` | `Float32Array` | 各周波数帯域の振幅（FFTデータ）。 |
| `complexSpectrum` | `object` | FFTの複素数表現（実部と虚部）。 |

周波数データを使用した例：
```javascript
export function update(ctx, width, height, audio) {
  const bands = audio.amplitudeSpectrum;
  const barWidth = width / bands.length;

  ctx.fillStyle = 'white';
  for (let i = 0; i < bands.length; i++) {
    const barHeight = bands[i] * height;
    ctx.fillRect(i * barWidth, height - barHeight, barWidth, barHeight);
  }
}
```

## 状態管理

`update` 関数は毎フレーム呼び出されるため、関数の *内部* で宣言された変数はリセットされます。フレーム間で状態（カウンターやオブジェクトの位置など）を保持するには、関数の *外部* で変数を宣言してください。

```javascript
let rotation = 0;

export function update(ctx, width, height, audio) {
  rotation += audio.rms * 0.1; // 音量に基づいて回転

  ctx.save();
  ctx.translate(width / 2, height / 2);
  ctx.rotate(rotation);
  ctx.fillStyle = 'red';
  ctx.fillRect(-50, -50, 100, 100);
  ctx.restore();
}
```

## より良いビジュアルのためのヒント

1.  **正規化**: ほとんどのオーディオ特徴量は生の数値です。描画に役立つ値を得るために、定数を掛ける（例：`audio.rms * 500`）必要があることがよくあります。
2.  **スムージング**: オーディオデータは急激に変化することがあります。単純なイージング関数を使用して、動きを滑らかにすることを検討してください。
    `targetSize = audio.rms * 1000; currentSize += (targetSize - currentSize) * 0.1;`
3.  **アルファブレンディング**: `fillRect` で画面を完全にクリアする代わりに、半透明の長方形を描画して「残像」エフェクトを作成できます。
    `ctx.fillStyle = 'rgba(0, 0, 0, 0.1)'; ctx.fillRect(0, 0, width, height);`
