# GPUフィードバック

`Shekere.gpu`を使うと、rendererや生のrender targetをスケッチへ公開せずに、
時間とともに変化する画像・シミュレーション状態をGPU上へ保持できます。
波紋、蓄積、煙のような表現、texture-state particleなどに利用できます。

## passの作成と更新

passはmodule評価中、`setup`、または後続の`update`で作成できます。`build`は
作成時に一度だけ実行され、次の状態を計算するTSL nodeを返します。

```javascript
this.feedback = Shekere.gpu.createFeedbackPass({
  name: "fade",
  width: 320,
  height: 180,
  format: "rgba16f",
  textures: ["seed"],
  uniforms: { decay: 0.97 },
  clearValue: [0, 0, 0, 0],
  build({ previous, textures, uniforms, uv }) {
    return TSL.max(
      previous.sample(uv).mul(uniforms.decay),
      textures.seed.sample(uv)
    );
  }
});

export function update() {
  this.feedback.update({
    textures: { seed: Shekere.camera.motion.maskNode },
    uniforms: { decay: 0.98 }
  });
}
```

`update()`は現在のsketch frameに対して1回の実行を予約します。同じframeで
複数回呼ぶと、最後に検証を通った値へ集約されます。予約されなかったpassは
offscreen描画を消費せず、以前の状態を保持します。停止後の`deltaTime`は
最大0.1秒です。

## 入力と依存関係

texture入力には`THREE.Texture`、TSL texture node、先に作成した
`FeedbackPass`、または`null`を指定できます。未指定と`null`は黒いfallbackを
参照します。他のpassへ依存するときは、作成順を検証できるように`node`ではなく
`FeedbackPass`自体を渡してください。後から作成したpassへの依存や任意のgraph
並べ替えはサポートしません。

uniformは有限のscalar、または2～4要素の数値配列です。名前と次元は作成時に
固定されます。不明な名前、異なる次元、非有限値を含む更新は全体が拒否され、
以前の正しい入力が維持されます。

## samplingと所有権

TSLでは`pass.node`を使用してください。ping-pong textureが交換されてもnodeの
同一性は維持されます。`pass.texture`は現在の生出力で、実行ごとに変わる場合が
あり、disposeまたは失敗後は`null`になります。

`build`の`uv`はoffscreen pass用の正規化UVです。scene背景へ表示するときは
screen UVを使います。

```javascript
const screenUv = TSL.screenUV.flipY();
scene.backgroundNode = this.feedback.node.sample(screenUv).rgb;
```

render target、material、node、fallback textureはShekere所有です。変更・dispose
しないでください。`clear()`は両方の履歴targetを`clearValue`へ戻す処理を予約し、
`dispose()`は複数回呼んでも安全です。reload、setup失敗、sketch切替、Visualizer
終了時には、残っているpassもホストが解放します。

## 制限と実行順

- 幅・高さ: `1–1024`の整数
- live pass: sketchあたり最大`8`
- 論理pixel数: ping-pong複製前でsketchあたり最大`2,097,152`
- format: 既定は`rgba8`、対応backendのみ`rgba16f`
- filtering: linear、mipmapとcolor-space変換なし

passはcamera binding、camera motion解析、sketchの`update`の後、main sceneと
post-processing描画の前に実行されます。1つのpassが失敗しても、そのpassだけを
無効化し、sketchと他のpassは継続します。

サンプルは[`camera_motion_ripple.js`](https://github.com/katk3n/shekere/blob/main/examples/camera_motion_ripple.js)と
[`feedback_particles.js`](https://github.com/katk3n/shekere/blob/main/examples/feedback_particles.js)を参照してください。
