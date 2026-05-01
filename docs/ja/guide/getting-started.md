# はじめに

Shekereへようこそ！このガイドでは、数分で最初のオーディオ・リアクティブなビジュアルを動かす手順を説明します。

## インストール

### バイナリをダウンロードする（推奨）
[GitHub Releases](https://github.com/katk3n/shekere/releases) ページから、お使いのOSに合わせた最新バージョンをダウンロードできます。
- **macOS**: `.dmg` ファイルをダウンロードして開き、Shekereを「アプリケーション」フォルダにドラッグ＆ドロップしてください。

### ソースからビルドする
自分でビルドしたい場合は、**Node.js** (v20以上) と **Rust** がインストールされている必要があります。
```bash
# リポジトリをクローン
git clone https://github.com/katk3n/shekere.git
cd shekere

# 依存関係のインストール
npm install

# 開発モードで実行
npm run tauri dev
```

## 初回起動

Shekereを最初に起動したとき、OSから **「マイクへのアクセス許可」** を求められます。
- **重要**: Shekereが音声を解析してビジュアルを反応させるために、この許可は必須です。Shekereが音声を録音したり送信したりすることはありません。解析はすべてローカルで行われます。

## サンプルスケッチを読み込む

Shekereには、すぐに試せるサンプルスケッチがいくつか同梱されています。
1. Shekereを起動します。
2. **Control Panel**（コントロールパネル）ウィンドウで、**"Open Sketch"** ボタンをクリックします。
3. Shekereのソースフォルダ内にある `examples/` ディレクトリに移動します。
4. `hello_world.js` や `spectrum.js` を選択します。
5. **Visualizer** ウィンドウにビジュアルが表示され、マイクが拾った音に反応し始めるはずです。

## 最初のスケッチを作成する

Shekereのスケッチ作成は、1つのJavaScript関数を書くだけで完了します。以下のコードを `my_first_sketch.js` として保存してください。

```javascript
// Shekereは 'update' という名前の関数を探します
export function update(ctx, width, height, audio) {
  // 背景をクリア
  ctx.fillStyle = 'black';
  ctx.fillRect(0, 0, width, height);

  // オーディオデータを使用（audio.rms は音量に相当します）
  const radius = audio.rms * 500;

  // 音に反応して大きさが変わる円を描画
  ctx.beginPath();
  ctx.arc(width / 2, height / 2, radius, 0, Math.PI * 2);
  ctx.fillStyle = 'cyan';
  ctx.fill();
}
```

コントロールパネルから **"Open Sketch"** をクリックし、作成したファイルを選択してください。これで、あなただけのオーディオ・リアクティブ・ビジュアルの完成です！

---

次へ: [スケッチの書き方](./writing-sketches.md) (準備中)
