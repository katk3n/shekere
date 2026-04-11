# Architecture Decision Record (ADR): 0003 Multiple Sketches Switching

## 1. Status

Implemented (v0.7.0)

## 2. Context & Goals (背景と目的)

現在の Shekere は、1度に1つのスケッチファイル (.js) を選択して読み込み、ホットリロード（監視）する仕組みとなっている。
しかし、ライブパフォーマンスやVJなどの実際のユースケースにおいては、複数のスケッチをあらかじめ読み込んでおき、パフォーマンスの進行に合わせて瞬時に切り替えられる（スイッチング）機能が必要である。
本 ADR では、Web/Tauri アーキテクチャの制約内で、効率的に複数ファイルを読み込み、MIDIコントローラやキーボード入力からスケッチを切り替えるためのアーキテクチャや状態管理の方法を定義する。

## 3. System Architecture Constraints (システムアーキテクチャの制約)

1. **State Management (状態管理)**
   - ファイルの管理や切り替えロジックは Control Panel (`App.tsx`) に集約する。
   - Visualizer ウィンドウ (`visualizer.ts`) は自身が何かを複数保持するのではなく、 Control Panel から送られてきた単一のコード（`user-code-update` イベント）を実行・破棄する従来の仕組みをそのまま使い回す。これにより余計なメモリリークのリスクを抑え、単一ファイル時と変わらない挙動を担保する。
2. **File Selection Method**
   - TOML ファイル（プレイリスト設定ファイル）を読み込んで一括設定する方式と、UI 上の各スロットからファイルダイアログ (`@tauri-apps/plugin-dialog`) を用いて個別に `.js` ファイルを選択する方式の両方をサポートする。
3. **Switching Triggers**
   - **Keyboard (PC)**: Control Panel ウィンドウでのキーボードショートカット。
   - **MIDI**: Visualizer および Control Panel が受信している MIDI イベント (`midi-event`) をトリガーとする（特定のNote番号やCC）。
   - **OSC**: Control Panel が受信している OSC イベント (`osc-event`) の Address をトリガーとする。

## 4. Proposed Design (提案される設計)

### 4.1 State (状態の保持)

Control Panel 側 (`App.tsx`) で以下の状態（State）を管理する。

- `playlist: Array<{ path: string, midiNote?: number, midiCc?: number, oscKey?: string, oscValue?: string }>`: スケッチファイルのパスと、そのスケッチを直接呼び出す（切り替える）ための引数（MIDI 信号や OSC の Key-Value ペアなど）のメタデータを保持するリスト。
- `currentIndex: number`: 現在アクティブなスケッチのインデックス（0開始）。
- `midiNavigation: { next?: { note?: number, cc?: number }, prev?: { note?: number, cc?: number } }`: TOMLから読み込んだ、MIDI信号による全体的な「次・前」の切り替え設定（オプション）。
- `oscNavigation: { next?: { key: string, value: string }, prev?: { key: string, value: string } }`: TOMLから読み込んだ、OSCの引数（TidalCycles のような key-value 形式）による「次・前」の切り替え設定（オプション）。

### 4.2 Loading & Watch Logic (読み込みと監視ロジック)

1. TOML などの設定ファイルを読み込むと、そこに定義された順番で `playlist` が構築される。もしくは手動でファイルを選択してスロットに追加する。
2. アクティブなファイル (`playlist[currentIndex].path`) が存在する場合、そのファイルを `readTextFile` で読み込み、 `user-code-update` イベントとして Visualizer に送信する。
3. ファイル監視 (`watch`) は、常に `playlist[currentIndex].path` のファイルに対してのみ行う。

### 4.3 Trigger Mechanisms (切り替えトリガー)

- **Keyboard Events**: `App.tsx` 内に `keydown` イベントリスナを登録。
   - `ArrowRight` (次へ), `ArrowLeft` (前へ), 数字キー `1`~`9` (ダイレクト選択)。
   - インデックスが末尾や先頭を越えた場合はループさせる。
- **MIDI Events**: `midi-event` に流れてくる信号（Note On/Off, Control Change）を監視する。
   - `midiNavigation` に設定された「次/前」の Note/CC が来た場合は、`currentIndex` を増減させる（末尾・先頭対応でループ処理）。
   - 各スケッチ (`playlist[i]`) に `midiNote` や `midiCc` が設定されており、受信した信号とマッチした場合は、そのインデックス `i` へダイレクトにジャンプ（切り替え）する。
- **OSC Events**: `osc-event` に流れてくる信号の引数（TidalCycles等の key-value 形式など）を監視する。
   - `oscNavigation.next` または `oscNavigation.prev` で指定された `key` と `value` に一致するペアが引数に含まれていた場合、`currentIndex` を増減させる（末尾・先頭対応でループ処理）。
   - 各スケッチ (`playlist[i]`) に `oscKey` および `oscValue` が設定されており、受信した引数内のペアとマッチした場合は、そのインデックス `i` へダイレクトにジャンプ（切り替え）する。

### 4.4 TOML Configuration Format (TOMLの設定フォーマット)

プレイリストの一括読み込みに用いる TOML ファイルのフォーマットは以下の通り。

```toml
[midi.navigation.next]
note = 36
cc = 10

[midi.navigation.prev]
note = 37
cc = 11

[osc.navigation.next]
key = "s"
value = "bd"

[osc.navigation.prev]
key = "s"
value = "cp"

[[sketch]]
file = "sketches/01_intro.js"
midi_note = 48
osc_key = "s"
osc_value = "hc"

[[sketch]]
file = "sketches/02_main.js"
midi_cc = 20
osc_key = "s"
osc_value = "sn"
```

### 4.5 UI Updates

Control Panel に「Playlist」UI を実装する。
リストにはファイルのパス（またはファイル名）と、割り当てられている MIDI 信号（例: `Note: 36`）が表示される。現在アクティブな行は視覚的にハイライトされ、どこがアクティブかを明示する。
また、TOML ファイルの読み込み機能 (Load Playlist TOML) ボタンスロットを設ける。

## 5. Alternatives Considered (他の検討手段)

- **Visualizer にすべてのコードを送って切り替える案**: イベントを Emit せず Visualizer 側に複数持たせる場合、WebGL や Three.js コンテキストでのコンフリクトや重いメモリ確保が発生しやすくなるため却下（一度に関数・オブジェクトが多数初期化されるリスク）。
- **全てのファイルを Watch する案**: 複数の一斉保存があった際に意図しないスケッチのリロードやイベント発火が起きるため、監視は「現在選択されているファイル」のみに限定するのが安定する。

