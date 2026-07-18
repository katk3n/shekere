# カメラ

Shekereでは、ライブカメラ映像をThree.jsの`VideoTexture`としてスケッチに
渡せます。キャプチャはVisualizerウィンドウ内だけで行われるため、映像フレームを
ウィンドウ間でシリアライズ、転送することはありません。

## カメラの開始

カメラは自動的には起動しません。Control Panelで次の操作を行います。

1. Cameraセクションで **Default Device** または使用するカメラを選択します。
2. **Enable Camera** をクリックし、表示された場合はカメラ権限を許可します。
3. 状態が`active`になったことと、実際の解像度・フレームレートを確認します。

権限を許可するまでは、デバイス名が一般的な名前で表示される場合があります。
キャプチャ中にデバイスを変更すると、指定したデバイスでキャプチャを再起動します。
指定したデバイスを開けない場合、別のカメラへ暗黙に切り替えずエラーを表示します。

## スケッチAPI

すべての`update(context)`呼び出しで`camera`プロパティを利用できます。

```javascript
export function update({ camera }) {
  if (this.material.map !== camera.texture) {
    this.material.map = camera.texture;
    this.material.needsUpdate = true;
  }
}
```

| プロパティ | 型 | 説明 |
| :--- | :--- | :--- |
| `camera.active` | `boolean` | ライブキャプチャが動作中かどうか。 |
| `camera.texture` | `THREE.VideoTexture \| null` | 現在のホスト所有カメラテクスチャ。 |
| `camera.width` | `number` | 実際のキャプチャ幅（ピクセル）。 |
| `camera.height` | `number` | 実際のキャプチャ高さ（ピクセル）。 |
| `camera.frameRate` | `number` | デバイスが報告した実際のフレームレート。 |

`camera`オブジェクトの同一性はフレーム間で維持されます。停止中または失敗時は
`active`が`false`、`texture`が`null`、すべての数値が`0`になります。

キャプチャの再起動やデバイス変更では`camera.texture`が交換される場合があります。
上記の例のように、マテリアルのテクスチャ参照を比較して更新してください。

::: warning テクスチャの所有権
`VideoTexture`はShekereが所有します。スケッチから
`camera.texture.dispose()`を呼び出さないでください。cleanupではスケッチ自身が
作成したジオメトリ、マテリアル、テクスチャだけを破棄してください。
:::

カメラのライフサイクルはスケッチから独立しています。スケッチの再読込や切替では
動作中のカメラは停止しません。不要になったら **Stop Camera** をクリックします。

## キャプチャ初期値とトラブルシューティング

Shekereは1280×720、30fpsを優先値として要求します。カメラによって別の対応形式が
選択される場合があり、実際の値はスケッチとControl Panelに公開されます。

Control Panelでは、権限拒否、デバイスなし・切断、制約非対応、メディアAPI非対応、
その他の取得・再生失敗を区別して表示します。カメラが失敗しても
`context.camera`は安全な停止状態になり、Visualizerの描画ループは継続します。
