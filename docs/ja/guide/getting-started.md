# はじめに

Shekereへようこそ！このガイドでは、数分で最初のオーディオ・リアクティブなビジュアルを動かす手順を説明します。

## インストール

### バイナリをダウンロードする（推奨）
[GitHub Releases](https://github.com/katk3n/shekere/releases) ページから、お使いのOSに合わせた最新バージョンをダウンロードできます。
- **macOS**: `.dmg` ファイルをダウンロードして開き、Shekereを「アプリケーション」フォルダにドラッグ＆ドロップしてください。
- **macOSでの初回起動**: 現在、アプリが署名されていないため、macOSによってデフォルトでブロックされます。開くには以下の手順が必要です：
  1. アプリを開き、警告ダイアログで **「OK」** をクリックします。
  2. **「システム設定」** > **「プライバシーとセキュリティ」** に移動します。
  3. **「セキュリティ」** セクションまでスクロールし、Shekereに対して **「このまま開く」** をクリックします。

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

### 🛡️ 権限の永続化 (macOS)
起動するたびにマイクやファイルへのアクセス許可を求められる場合、それはバイナリが「未署名」であるためです。macOSは、セキュリティ対策として未署名アプリの権限を再起動のたびにリセットします。

権限を永続化するには、ローカルでアプリを「再署名」します：
1. **Shekere.app** を `/Applications`（アプリケーション）フォルダに移動します。
2. **ターミナル** を開き、以下を実行します：
   ```bash
   # 1. クアランティン（隔離）フラグを解除
   xattr -cr /Applications/Shekere.app

   # 2. ローカルアイデンティティで再署名
   codesign --force --deep --sign - /Applications/Shekere.app
   ```

::: danger セキュリティ警告
バイナリの再署名は、macOS Gatekeeperによるチェックをバイパスします。公式リポジトリからダウンロードしたもの、または自分でビルドしたものに対してのみ行ってください。
:::

## サンプルスケッチを読み込む

Shekereにはサンプルスケッチがいくつか同梱されています。
1. Shekereを起動します。「コントロールパネル」と「ビジュアライザー」の2つのウィンドウが表示されます。
2. **Control Panel** ウィンドウで、**"Open Sketch"** ボタンをクリックします。
3. `examples/` ディレクトリに移動し、`audio_reactive_knot.js` や `spectrum.js` を選択します。
4. **Visualizer** ウィンドウにビジュアルが表示され、マイクの音に反応し始めます。

## 最初のスケッチを作成する

Shekereのスケッチ作成には **Three.js** を使用します。以下のコードを `my_first_sketch.js` として保存してください。

```javascript
export function setup(scene) {
  // 3Dオブジェクトのセットアップ
  const geometry = new THREE.IcosahedronGeometry(1, 2);
  const material = new THREE.MeshNormalMaterial({ wireframe: true });
  this.mesh = new THREE.Mesh(geometry, material);
  scene.add(this.mesh);
}

export function update({ time, audio }) {
  // 毎フレーム実行されます (~60fps)
  this.mesh.rotation.y = time * 0.5;
  
  // オーディオ音量（bass）に反応させる
  const s = 1 + audio.bass;
  this.mesh.scale.set(s, s, s);
}

export function cleanup(scene) {
  // メモリリークを防ぐためにシーンをクリア
  Shekere.clearScene(scene);
}
```

コントロールパネルから **"Open Sketch"** をクリックし、作成したファイルを選択してください。これで、あなただけの3Dオーディオ・リアクティブ・ビジュアルの完成です！

---

次へ: [スケッチの書き方](./writing-sketches.md)
