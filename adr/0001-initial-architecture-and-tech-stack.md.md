# Architecture Decision Record (ADR): shekere

次世代ライブコーディング用オーディオビジュアル環境の構築

## 1. Status

Accepted (承認済)

## 2. Context & Goals (背景と目的)

既存の「shekere (v1)」はRustとWGSLを用いた環境であったが、ユーザーの学習コストおよびホストアプリの開発・保守コストが高い課題があった。
本プロジェクト「shekere (v2)」は、以下の実現を目的とする。
- 参考 (既存のshekere(v1)のリポジトリ): https://github.com/katk3n/shekere-legacy

1. ターゲット層の拡張: 高度なシェーダー言語(WGSL)から、Web標準の JavaScript / Three.js へと記述言語をピボットし、学習コストを極限まで下げる。
2. 開発の高速化と安定化: 複雑なシステムプログラミングを避け、Web標準API と Tauri v2のプラグイン を最大限活用したモダンなハイブリッドアーキテクチャを採用する。

## 3. Core Paradigm (コア設計思想)

shekereは「固定のビジュアルを表示するアプリ」ではなく、 **「ユーザーが外部エディタで書いたJavaScriptコードを動的に読み込み、音楽と同期させてThree.jsで描画するホスト環境（ランナー）」** である。

## 4. System Architecture (システムアーキテクチャ)

- Desktop Framework: Tauri v2
- Build Tool: Vite
- Language:
    - TypeScript (UIおよびフロントエンドロジック)
    - Rust (TypeScript で実現できない機能の実装)

システムは以下の役割分担（境界）を厳密に守ること。

### 4.1 フロントエンド (TypeScript / WebView)

OSC受信以外のすべての処理をここに集約する。Rustのコード量は最小限に抑えること。

- UIフレームワーク: Viteエコシステムのモダンなライブラリ (React, Vue, または Vanilla TS)。
- 描画エンジン: Three.js (標準API + EffectComposerによるポストプロセス)。
- オーディオ解析: Web Audio API を使用 (AnalyserNode によるFFT解析や低音検知)。
- MIDI入力: Web MIDI API を使用。
- ファイル監視: Tauri v2公式プラグイン `@tauri-apps/plugin-fs` を使用し、TS側から直接ユーザーファイルの変更を監視（`watch`）し、読み込む（`readTextFile`）。

### 4.2 バックエンド (Rust / Tauri Core)

Webの制約でフロントエンドから直接実行できない処理のみを担当する。

- OSC通信: `rosc` クレート等を用い、UDPソケットでOSCメッセージを受信。受信データをTauriの `emit` でフロントエンドへ送信する。
- ※注意: Rustでのファイル監視（`notify`等）は行わない。

### 5. Multi-Window Design (マルチウィンドウ設計)

ライブパフォーマンス用途のため、必ず2つのウィンドウを分離して起動・管理する。

1. Control Panel (メインウィンドウ):
    - UIの描画、ファイルの選択（監視パスの決定）。
    - Web Audio API / Web MIDI APIのインスタンスを保持し、データの解析を行う。
    - 取得・解析したデータを、TauriのIPC (`emit`) を通じて毎フレーム Visualizer へ送信する。
2. Visualizer (描画用サブウィンドウ):
    - フルスクリーン出力を想定したUIを持たないウィンドウ。
    - Three.jsのCanvasを保持する。
    - Control PanelからのデータおよびOSCデータ (listen) を受け取り、ユーザーコードにデータを注入して描画を更新する。

### 6. Dynamic Module Loading Strategy (ユーザーコードの動的実行)

Viteのバンドル制約を回避し、ローカルにある生のユーザーJSファイルを安全にホットリロードするため、 **Blob URLパターン** を採用する。

```typescript
// エージェントへの実装ヒント: Blob URLを使ったホットリロード
const codeString = await readTextFile(userFilePath); // Tauri fs pluginで読み込み
const blob = new Blob([codeString], { type: 'application/javascript' });
const blobUrl = URL.createObjectURL(blob);

if (currentModule) currentModule.cleanup(); // 既存のThree.jsオブジェクトの破棄等

// ユーザーのモジュールを動的インポートし、Three.jsのsceneを渡す
const userModule = await import(/* @vite-ignore */ blobUrl);
userModule.setup(scene);
```

### 7. Interface Contract (ユーザーコードとの規約)

ホストアプリ（shekere）とユーザーコード間のインターフェース仕様。AIエージェントはこれに適合するローダーを実装すること。

```typescript
// ユーザーが記述するJSファイル (例: my_sketch.js) 
export function setup(scene) {
    this.mesh = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshNormalMaterial());
    scene.add(this.mesh);
}

export function update(context) {
    const { time, audio, midi, osc } = context;
    // 例: 低音(audio.bass)でスケール変化
    const scale = 1.0 + (audio.bass * 2.0);
    this.mesh.scale.set(scale, scale, scale);
}

// 必須: ホットリロード時のメモリリークを防ぐためのクリーンアップ関数
export function cleanup(scene) {
    scene.remove(this.mesh);
    this.mesh.geometry.dispose();
    this.mesh.material.dispose();
}
```

### 8. Implementation Roadmap for AI Agent (実装指示フェーズ)

AIエージェントは以下のフェーズに従って段階的に実装・確認を進めること。

#### Phase 1: Bootstrapping

- `create-tauri-app` 等を用いたTauri v2 + Vite + TS基盤の構築。
- Control PanelとVisualizerの2つのウィンドウが起動する設定 (`tauri.conf.json` 等) の実装。

#### Phase 2: File Watching & Dynamic Loading

- `@tauri-apps/plugin-fs` を用いたJSファイルの監視機構の実装。
- Visualizer側での Three.js 基盤構築と、Blob URLを用いたJSファイルの動的実行機能の実装。

#### Phase 3: Data Pipeline (Audio & OSC)

- Control Panelでのマイク入力許可とWeb Audio API (FFT) 実装。
- Rust側でのUDPソケットによるOSC受信とフロントエンドへの emit 実装。
- 各種データを `update(context)` へ注入するパイプラインの結合。

#### Phase 4: Post-Processing & UI

- Visualizerへの EffectComposer (`UnrealBloomPass`等) の導入。
- Control Panel側でのパラメーター調整UI（Bloom強度など）の実装とウィンドウ間同期。

### 9. Constraints (厳格な制約事項 - 遵守必須)

- Rustのオーディオ/MIDIクレート使用禁止: `cpal`, `rustfft`, `midir` などは絶対に `Cargo.toml` に追加しないこと。すべてWeb Audio/Web MIDI APIで実装する。
- Rustのファイル監視クレート使用禁止: `notify` などのファイル監視クレートは使用しない。`@tauri-apps/plugin-fs` をフロントエンドから呼び出すこと。
- 自作コンパイラの禁止: WGSLやGLSLのパース、コンパイル処理などを自作・実装しようとしないこと。ユーザーコードは純粋なJavaScriptとして扱う。