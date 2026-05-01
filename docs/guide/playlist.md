# Playlist

Playlists allow you to manage multiple sketches and switch between them seamlessly. This is ideal for live performances where you need to move through different visual concepts.

## TOML Format

Playlists are defined using the [TOML](https://toml.io/) format. A playlist file consists of global navigation settings and an array of individual sketches.

### Global Navigation

You can set up global triggers to move to the "Next" or "Previous" sketch in your list.

```toml
[midi.navigation.next]
note = 38 # Trigger "Next" when MIDI Note 38 is pressed

[midi.navigation.prev]
note = 36 # Trigger "Previous" when MIDI Note 36 is pressed

[osc.navigation.next]
key = "s"
value = "bd" # Trigger "Next" on OSC /dirt/play where s="bd"
```

### Sketch Configuration

Each sketch is defined as a `[[sketch]]` entry.

| Property | Description |
| :--- | :--- |
| `file` | The path to the `.js` sketch file (relative to the TOML file). |
| `midi_note` | (Optional) MIDI note number to jump directly to this sketch. |
| `osc_key` / `osc_value` | (Optional) OSC argument pair to jump directly to this sketch. |

## Complete Example

Save the following as `my_playlist.toml`:

```toml
# Navigation
[midi.navigation.next]
note = 38
[midi.navigation.prev]
note = 36

# First Sketch
[[sketch]]
file = "intro.js"
midi_note = 48
osc_key = "s"
osc_value = "intro"

# Second Sketch
[[sketch]]
file = "visuals/glitch.js"
midi_note = 49
osc_key = "s"
osc_value = "glitch"

# Third Sketch
[[sketch]]
file = "visuals/ambient.js"
midi_note = 50
```

## How to Load

1. Launch Shekere.
2. In the Control Panel, click the **"Load Playlist"** button.
3. Select your `.toml` file.
4. Use your mapped MIDI notes, OSC triggers, or the **Arrow Keys** (Left/Right) on your keyboard to switch between sketches.
