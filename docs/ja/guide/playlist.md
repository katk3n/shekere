# プレイリスト

プレイリストを使用すると、複数のスケッチを管理し、それらをシームレスに切り替えることができます。これは、異なるビジュアル・コンセプトを次々と展開する必要があるライブパフォーマンスに最適です。

## TOML フォーマット

プレイリストは [TOML](https://toml.io/ja/) 形式で定義します。プレイリストファイルは、グローバルなナビゲーション設定と、個々のスケッチの配列で構成されます。

### グローバルナビゲーション

リスト内の「次」または「前」のスケッチに移動するためのグローバルなトリガーを設定できます。

```toml
[midi.navigation.next]
note = 38 # MIDIノート38が押されたときに「次へ」

[midi.navigation.prev]
note = 36 # MIDIノート36が押されたときに「前へ」

[osc.navigation.next]
key = "s"
value = "bd" # OSC /dirt/play で s="bd" の時に「次へ」
```

### スケッチの構成

各スケッチは `[[sketch]]` エントリとして定義します。

| プロパティ | 説明 |
| :--- | :--- |
| `file` | `.js` スケッチファイルへのパス（TOMLファイルからの相対パス）。 |
| `midi_note` | (オプション) このスケッチに直接ジャンプするためのMIDIノート番号。 |
| `osc_key` / `osc_value` | (オプション) このスケッチに直接ジャンプするためのOSC引数のペア。 |

## 完全な例

以下を `my_playlist.toml` として保存してください：

```toml
# ナビゲーション
[midi.navigation.next]
note = 38
[midi.navigation.prev]
note = 36

# 1番目のスケッチ
[[sketch]]
file = "intro.js"
midi_note = 48
osc_key = "s"
osc_value = "intro"

# 2番目のスケッチ
[[sketch]]
file = "visuals/glitch.js"
midi_note = 49
osc_key = "s"
osc_value = "glitch"

# 3番目のスケッチ
[[sketch]]
file = "visuals/ambient.js"
midi_note = 50
```

## 読み込み方法

1. Shekereを起動します。
2. コントロールパネルで **"Load Playlist"** ボタンをクリックします。
3. 作成した `.toml` ファイルを選択します。
4. マッピングしたMIDIノート、OSCトリガー、またはキーボードの **矢印キー** (左右) を使ってスケッチを切り替えることができます。
