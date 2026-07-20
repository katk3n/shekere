# カメラモーション

Shekereは連続するカメラフレームをGPU上で比較し、現在のモーションマスクと、
時間減衰するモーショントレイルをスケッチへ公開できます。aura、残像、reveal、
distortion、displacement、geometry maskなどのエフェクトに利用できます。

検出対象は画像内のあらゆる変化です。姿勢推定、腕トラッキング、人物セグメンテーション、
物体認識ではありません。カメラ移動、照明・露出の変化、動く背景もモーションになります。

## 解析の有効化

モーション解析はGPU render targetと複数のoffscreen passを使用するため、
opt-inです。`setup(scene)`が返すオブジェクトで要求します。

```javascript
export function setup(scene) {
  return {
    camera: {
      motion: {
        enabled: true,
        threshold: 0.08,
        blur: 6,
        decay: 0.94
      }
    }
  };
}
```

| 設定 | 初期値 | 範囲 | 説明 |
| :--- | :--- | :--- | :--- |
| `enabled` | `false` | boolean | アクティブなスケッチのGPU解析を有効にします。 |
| `threshold` | `0.08` | `0.0–1.0` | モーションと判定する最小輝度差です。 |
| `blur` | `6` | `0–20` | 解析ピクセル単位のGaussian blur半径です。 |
| `decay` | `0.94` | `0.0–0.999` | カメラフレームごとに維持する前回トレイルの割合です。 |

範囲外の値はclampされます。`camera.motion`を省略するか、
`enabled: false`を指定すると解析を停止し、GPUリソースを解放します。

## 安定したTSL node

TSL graphは通常、最初の`update(context)`より前の`setup(scene)`で構築します。
そのためShekereは、同一性が安定したホスト所有nodeをグローバル名前空間で提供します。

```javascript
export function setup(scene) {
  const trail = Shekere.camera.motion.trailNode.sample(TSL.uv()).r;
  const color = TSL.uniform(new THREE.Color(0x35a7ff));

  this.material = new THREE.MeshBasicNodeMaterial();
  this.material.colorNode = color.mul(trail);
}
```

`maskNode`と`trailNode`の同一性は維持され、Shekereが内部textureを更新します。
解析停止中または初期化中は黒いfallback textureをサンプルします。スケッチ側で
fallbackを作成したり、ping-pong切替後にtexture nodeを再設定する必要はありません。

::: warning Nodeの所有権
`Shekere.camera.motion.maskNode`、`trailNode`、およびfallback textureは
Shekereが所有します。サンプルには利用できますが、`value`の代入やdisposeを
行わないでください。
:::

## モーションデータ

すべての`update(context)`呼び出しで`camera.motion`を利用できます。

```javascript
export function update({ camera, audio, bloom }) {
  this.mesh.visible = camera.motion.active;
  bloom.strength = 0.5 + audio.bass * 3;
}
```

| プロパティ | 型 | 説明 |
| :--- | :--- | :--- |
| `active` | `boolean` | 完成したモーションテクスチャを利用できるか。 |
| `maskTexture` | `THREE.Texture \| null` | 最新解析フレームのblur済みモーションです。 |
| `trailTexture` | `THREE.Texture \| null` | 時間減衰しながら蓄積した最近のモーションです。 |
| `width` | `number` | 解析テクスチャの幅です。 |
| `height` | `number` | 解析テクスチャの高さです。 |

解析解像度の長辺は320ピクセルです。新しいカメラフレームごとに最大1回、
スケッチのupdateより前に解析します。最初のフレームでは履歴を初期化するため、
フレーム比較が可能になるまで`active`はfalseです。

`camera.motion`のオブジェクト同一性は維持されます。カメラ再起動、デバイス変更、
キャプチャ寸法変更、trailのping-pong更新ではraw texture参照が交換される場合が
あります。TSL graphでは安定したShekere nodeを使用してください。raw textureが
必要なconsumerは参照変更時に更新してください。

::: warning テクスチャの所有権
モーションテクスチャはShekereが所有します。スケッチから`maskTexture`や
`trailTexture`をdisposeしないでください。スケッチ自身が作成したリソースだけを
破棄してください。
:::

音声反応するTSL auraの例は
[`examples/camera_motion_aura.js`](https://github.com/katk3n/shekere/blob/main/examples/camera_motion_aura.js)
を参照してください。

独立した速度や寿命を持つparticle、成長するripple、smoke、反復simulation stateは、
モーションを入力にしたホスト管理の[GPU feedback pass](./gpu-feedback.md)で実装できます。
