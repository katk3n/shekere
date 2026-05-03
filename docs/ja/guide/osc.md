# OSC

Shekereは、TidalCycles、Sonic Pi、TouchDesignerなどの他のアプリケーションから **OSC (Open Sound Control)** メッセージを受信できます。

## 接続の詳細

- **デフォルトポート**: `2020`
- **プロトコル**: UDP

## スケッチでのOSCの処理

OSCデータは、用途に応じて **ステート** (最新の値) または **イベント** (トリガー) の2つの方法で処理できます。

### 1. 永続的なステート (`osc`)
`osc` オブジェクトには、特定のアドレスで受信した最新のデータが保存されます。これは、フェーダーやXYパッドなどの連続的なコントロールに最適です。

```javascript
export function update({ osc }) {
  // /fader1 から最新の値を取得（数値であることを想定）
  const faderValue = osc['/fader1'] || 0;
  this.mesh.position.x = faderValue * 10;
}
```

### 2. 個別のイベント (`oscEvents`)
`oscEvents` 配列には、**現在のフレーム内** で受信したすべてのOSCメッセージが含まれています。これは、ドラムのビートやワンショットのイベントなどのトリガーに最適です。

```javascript
export function update({ oscEvents }) {
  oscEvents.forEach(event => {
    if (event.address === '/beat') {
      // ビートごとにビジュアルの変化をトリガーする
      this.triggerFlash();
    }
  });
}
```

## 特別なサポート: TidalCycles

Shekereには、`/dirt/play` に送信される **TidalCycles** (SuperDirt) メッセージ専用のビルトイン・パーサーが含まれています。

TidalCyclesのメッセージは引数の配列ではなく、内部キー（`s`, `n`, `gain`, `cutoff` など）を使用した扱いやすいJavaScriptオブジェクトに自動的に変換されます。

```javascript
export function update({ oscEvents }) {
  oscEvents.forEach(({ address, data }) => {
    if (address === '/dirt/play') {
      // 'data' は { s: "bd", gain: 1, ... } のようなオブジェクトになっています
      if (data.s === 'bd') {
        this.kickEffect();
      }
    }
  });
}
```

## OSC使用のヒント

1.  **UDPトラフィック**: 高頻度のOSCデータは、UDPの特性上、ドロップしたり遅延したりすることがあります。正確な同期が必要な場合は、1フレームあたりに送信するメッセージ数を最小限に抑えるようにしてください。
2.  **デバッグ**: 受信しているOSCメッセージやそのアドレスの構造を確認するには、Shekereの **コントロールパネル（Control Panel）** 内の **Monitors** セクションを確認してください。
3.  **ポートマッピング**: 別のポートでOSCを受信する必要がある場合は、アプリケーションのポートが2020に固定されているため、OSCルーティングツールなどの外部プロキシを使用する必要があります。
