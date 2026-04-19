# Architecture Decision Record (ADR): 0003 Multiple Sketches Switching

## 1. Status

Implemented (v0.7.0)

## 2. Context & Goals

Currently, Shekere allows selecting and loading one sketch file (.js) at a time for hot-reloading (monitoring).
However, in actual use cases such as live performances and VJing, it is necessary to preload multiple sketches and be able to switch between them instantaneously as the performance progresses.
This ADR defines the architecture and state management methods for efficiently loading multiple files and switching sketches via MIDI controllers or keyboard input within the constraints of the Web/Tauri architecture.

## 3. System Architecture Constraints

1. **State Management**
   - File management and switching logic are centralized in the Control Panel (`App.tsx`).
   - The Visualizer window (`visualizer.ts`) remains a stateless consumer; it does not maintain multiple sketches itself. Instead, it continues to use the existing mechanism of executing or disposing of a single block of code (`user-code-update` event) sent from the Control Panel. This minimizes the risk of memory leaks and ensures behavior consistent with single-file mode.
2. **File Selection Method**
   - Supports both bulk configuration by loading a TOML file (playlist configuration file) and individual selection of `.js` files from slots in the UI using the file dialog (`@tauri-apps/plugin-dialog`).
3. **Switching Triggers**
   - **Keyboard (PC)**: Keyboard shortcuts within the Control Panel window.
   - **MIDI**: Triggered by MIDI events (`midi-event`) received by the Visualizer and Control Panel (specific Note numbers or CC).
   - **OSC**: Triggered by the Address or arguments of OSC events (`osc-event`) received by the Control Panel.

## 4. Proposed Design

### 4.1 State Management

The following states are managed on the Control Panel side (`App.tsx`):

- `playlist: Array<{ path: string, midiNote?: number, midiCc?: number, oscKey?: string, oscValue?: string }>`: A list holding sketch file paths and metadata (MIDI signals, OSC key-value pairs, etc.) for directly calling (switching to) that sketch.
- `currentIndex: number`: The current active sketch index (0-indexed).
- `midiNavigation: { next?: { note?: number, cc?: number }, prev?: { note?: number, cc?: number } }`: Optional global "Next/Previous" switching settings via MIDI signals loaded from TOML.
- `oscNavigation: { next?: { key: string, value: string }, prev?: { key: string, value: string } }`: Optional global "Next/Previous" switching settings via OSC arguments (key-value format like TidalCycles) loaded from TOML.

### 4.2 Loading & Watch Logic

1. When a configuration file like TOML is loaded, the `playlist` is constructed in the defined order. Alternatively, files are manually selected and added to slots.
2. If an active file (`playlist[currentIndex].path`) exists, that file is read via `readTextFile` and sent to the Visualizer as a `user-code-update` event.
3. File monitoring (`watch`) is always performed only for the currently active file (`playlist[currentIndex].path`).

### 4.3 Trigger Mechanisms

- **Keyboard Events**: Register a `keydown` event listener within `App.tsx`.
   - `ArrowRight` (Next), `ArrowLeft` (Previous), Number keys `1`~`9` (Direct selection).
   - Wrap around if the index exceeds the beginning or end of the list.
- **MIDI Events**: Monitor signals (Note On/Off, Control Change) flowing into `midi-event`.
   - If a "Next/Prev" Note/CC specified in `midiNavigation` is received, increment/decrement `currentIndex` (with wrap-around).
   - If a signal matches the `midiNote` or `midiCc` defined for a specific sketch (`playlist[i]`), jump directly to that index `i`.
- **OSC Events**: Monitor argument pairs (e.g., TidalCycles key-value format) flowing into `osc-event`.
   - If a pair matching `oscNavigation.next` or `oscNavigation.prev` is found in the arguments, increment/decrement `currentIndex` (with wrap-around).
   - If a pair matches the `oscKey` and `oscValue` defined for a specific sketch (`playlist[i]`), jump directly to that index `i`.

### 4.4 TOML Configuration Format

The format for the TOML file used for bulk playlist loading is as follows:

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

Implement a "Playlist" UI in the Control Panel.
The list displays the file path (or filename) and assigned MIDI signals (e.g., `Note: 36`). The currently active row is visually highlighted to indicate which sketch is live.
Additionally, provide a button slot for the "Load Playlist TOML" function.

## 5. Alternatives Considered

- **Sending all code to the Visualizer for switching**: Rejected because maintaining multiple sketch contexts (WebGL/Three.js) in the Visualizer could lead to resource conflicts and heavy memory allocation (risk of many functions/objects being initialized at once).
- **Watching all files**: Rejected because simultaneous saves across multiple files could cause unintended sketch reloads or event triggers. Monitoring only the "currently selected file" ensures stability.
