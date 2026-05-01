# オーディオ

Shekereは音声をリアルタイムで処理し、シンプルな音量レベルから高度なスペクトル特徴までをスケッチに提供します。

## 基本的なオーディオプロパティ

`update` 関数に渡される `audio` オブジェクトには、異なる周波数範囲を表す正規化された値（0.0 〜 1.0）が含まれています。

| プロパティ | 説明 |
| :--- | :--- |
| `volume` | 信号全体の平均的な音量。 |
| `bass` | 低域（250 Hz以下）の平均エネルギー。 |
| `mid` | 中域（250 Hz 〜 2000 Hz）の平均エネルギー。 |
| `high` | 高域（2000 Hz以上）の平均エネルギー。 |

### 例：基本的なリアクティビティ
```javascript
export function update({ audio }) {
  // 低音（bass）を使用して球体のサイズを制御する
  const scale = 1 + audio.bass * 2;
  this.mesh.scale.set(scale, scale, scale);
}
```

## 周波数帯域 (FFT)

`audio.bands` は **256個のビン** を持つ `Float32Array` で、低域から高域までの周波数スペクトルを表します。人間の聴覚に合わせて対数スケールでスケーリングされています。

### 例：スペクトル・ビジュアライザー
```javascript
export function update({ ctx, width, height, audio }) {
  const barWidth = width / audio.bands.length;
  ctx.fillStyle = 'white';
  
  audio.bands.forEach((value, i) => {
    const barHeight = value * height;
    ctx.fillRect(i * barWidth, height - barHeight, barWidth, barHeight);
  });
}
```

## 高度な特徴量 (Meyda)

より洗練された解析を行うには、`audio.features` オブジェクトを使用します。これらは Meyda ライブラリによって計算されます。

| 特徴量 | 型 | ユースケース |
| :--- | :--- | :--- |
| `rms` | `number` | ルート平均二乗。`volume` よりも正確な知覚音量を表します。 |
| `zcr` | `number` | ゼロ交差率。パーカッシブな音やノイズのような音の検出に便利です。 |
| `energy` | `number` | 信号の総エネルギー。 |
| `spectralCentroid` | `number` | スペクトルの重心。音の「明るさ」を示します。 |
| `spectralFlatness` | `number` | 純音 (0.0) かノイズ (1.0) かを判別します。 |
| `chroma` | `number[12]` | 12音階（C, C#, Dなど）ごとの強度。旋律や和音への反応に便利です。 |
| `mfcc` | `number[13]` | メル周波数ケプストラム係数。音色やスペクトル形状を表します。 |

### 例：パーカッション検出
```javascript
export function update({ audio }) {
  // 非常にパーカッシブな音（高いZCR）の場合にフラッシュを発生させる
  if (audio.features.zcr > 50) {
    this.flash = 1.0;
  }
  this.flash *= 0.9; // フラッシュを減衰させる
}
```

## 設定

`setup` 関数内で `audio` オブジェクトを返すことで、解析する周波数範囲をカスタマイズできます。

```javascript
export function setup(scene) {
  return {
    audio: {
      minFreqHz: 40,   // デフォルト: 27.5
      maxFreqHz: 8000  // デフォルト: 4186
    }
  };
}
```
