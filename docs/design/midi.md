# MIDI Integration Specification

## Overview

MIDI integration enables real-time control through MIDI input devices. This system provides access to MIDI messages for creating interactive audio-visual experiences.

## Purpose

Receives MIDI messages from connected MIDI devices for real-time parameter control and musical interaction.

## Features

- **Device Input**: Support for MIDI input devices (keyboards, controllers, etc.)
- **Real-time Processing**: Immediate response to MIDI messages
- **Parameter Control**: Map MIDI data to shader parameters
- **Musical Interaction**: Create music-responsive visual effects
- **Note On Attack Detection**: Detect the exact moment when notes are triggered

## Configuration

```toml
[midi]
enabled = true
```

### Configuration Parameters

- `enabled`: Boolean flag to enable/disable MIDI input processing

## GPU Integration

### Uniform Data Creation
- MIDI data is processed and converted to GPU-accessible uniform buffers
- Data is made available to shaders through binding group 2 (Sound Uniforms)
- Real-time updates ensure smooth audio-visual synchronization

### vec4 Packing Optimization
- All MIDI uniforms use vec4 packing for WebGPU alignment
- Ensures optimal memory layout for GPU processing
- Improves performance by reducing memory bandwidth requirements

## Shader Integration

### Sound Uniforms (Binding Group 2)
When MIDI is configured, shaders can access:
- **MIDI Data**: Real-time MIDI message values
- Note data, velocity information, and control changes
- **Note On Attack Data**: Instantaneous Note On detection for attack-responsive effects

### Helper Functions
Shaders can use helper functions for MIDI data access. For detailed function signatures and usage, see [API Reference](api-reference.md).

## Real-Time Processing

### Performance Considerations
- MIDI processing runs on separate threads to avoid blocking rendering
- Efficient data structures for real-time message processing
- Minimal latency between MIDI input and visual output

### Synchronization
- MIDI data is synchronized with the render loop
- Thread-safe data structures for concurrent access
- Consistent frame timing for smooth audio-visual effects

## MIDI Protocol

### Message Types
- Support for standard MIDI message types (Note On/Off, Control Change, etc.)
- Message parsing and validation
- Device connection management
- **Note On Detection**: Instantaneous capture of Note On messages with velocity information

### Device Management
- Automatic detection of connected MIDI devices
- Handle device connection/disconnection gracefully
- Support for multiple MIDI input devices

## Note On Attack Detection

### Purpose
The Note On attack detection feature provides instantaneous detection of when notes are triggered, enabling shader effects that respond specifically to the attack (onset) of musical notes rather than their sustained state.

### Data Structure
The MIDI uniform includes a dedicated `note_on` array that contains:
- **128 note slots**: One for each MIDI note (0-127)
- **Velocity values**: Normalized from 0.0-1.0 (original MIDI range 0-127)
- **Frame-based operation**: Values are only non-zero for the exact frame when Note On occurs

### Behavior
- **Note On Message**: When a Note On message is received, the corresponding note slot is set to the normalized velocity value
- **Frame Clear**: At the start of each frame, all note_on values are cleared to 0.0
- **Single Frame Duration**: Attack values are only available for one render frame
- **Independent of Note State**: Attack detection operates independently of the sustained note state

### Use Cases
- **Percussion Effects**: Trigger visual explosions or flashes on drum hits
- **Attack Synchronization**: Sync visual elements precisely with note onsets
- **Rhythmic Patterns**: Create effects that respond only to new note attacks, not sustained notes
- **Transient Detection**: Distinguish between attack and sustained portions of notes

### Shader Access
Use the `MidiNoteOn()` helper function to access attack data:
```wgsl
let attack_strength = MidiNoteOn(60u); // Middle C attack detection
if attack_strength > 0.0 {
    // Visual effect triggered only on note attack
}
```

### Difference from Standard Note Detection
- **MidiNote()**: Returns velocity while note is being held (sustained state)
- **MidiNoteOn()**: Returns velocity only for the frame when note attack occurs (instantaneous detection)

## MIDI History

### Overview
The MIDI History feature provides access to historical MIDI data from previous frames, enabling fade-out effects and temporal visual patterns without requiring multi-pass shaders.

### Purpose
- **Temporal Effects**: Create fade-out, echo, and trail effects using historical MIDI data
- **Simplified Implementation**: Avoid complex multi-pass shader setups for temporal effects
- **Enhanced Creative Control**: Access to 512 frames of MIDI history for rich visual programming

### Technical Architecture

#### Storage Buffer Migration
To accommodate the increased data volume (768KB for 512 frames of history), the MIDI system migrates from Uniform Buffers to Storage Buffers:
- **Data Capacity**: Storage buffers support large structured data arrays (768KB vs ~64KB limit for uniforms)
- **Performance**: Efficient access to large historical datasets
- **WebGPU Compatibility**: Full cross-platform support

#### Data Structure Design

**CPU Side (Rust)**: Ring buffer for memory efficiency
```rust
struct MidiHistoryData {
    // Ring buffer: 512 frames × 128 values × 3 arrays
    notes_history: [[f32; 128]; 512],
    controls_history: [[f32; 128]; 512],
    note_on_history: [[f32; 128]; 512],
    current_index: usize,  // Current frame position in ring buffer
}
```

**GPU Side (WGSL)**: Linear array for simple access
```wgsl
struct MidiHistory {
    // Linear arrays: 512 frames × 32 vec4s (128 values) × 3 arrays = 768KB
    notes_history: array<array<vec4<f32>, 32>, 512>,
    controls_history: array<array<vec4<f32>, 32>, 512>,
    note_on_history: array<array<vec4<f32>, 32>, 512>,
}
```

#### Ring Buffer Management
- **CPU Efficiency**: Ring buffer minimizes memory allocation overhead
- **GPU Simplicity**: Linear array transfer for straightforward shader access
- **Frame Updates**: Each render frame advances ring buffer and transfers current state to GPU

### Shader API

#### Updated Helper Functions
All MIDI functions now require a history parameter for consistency:

```wgsl
// New unified API (breaking change)
fn MidiNote(note_num: u32, history: u32) -> f32     // history = 0 for current frame
fn MidiControl(cc_num: u32, history: u32) -> f32    // history = 1-511 for past frames
fn MidiNoteOn(note_num: u32, history: u32) -> f32   // bounds-checked access
```

#### History Parameter
- **Range**: 0-511 (0 = current frame, 511 = oldest available frame)
- **Bounds Checking**: Invalid history values return 0.0
- **Frame Alignment**: History aligned with render frames (not absolute time)

### Use Cases

#### Fade-out Effects
```wgsl
// Create velocity fade trail
let current = MidiNote(60u, 0u);           // Current Middle C
let fade1 = MidiNote(60u, 10u) * 0.8;     // 10 frames ago, 80% intensity
let fade2 = MidiNote(60u, 20u) * 0.6;     // 20 frames ago, 60% intensity
let combined = max(current, max(fade1, fade2));
```

#### Echo Effects
```wgsl
// Rhythmic echo patterns (assuming 60fps)
let attack = MidiNoteOn(60u, 0u);         // Current attack
let echo1 = MidiNoteOn(60u, 30u) * 0.5;  // Echo at 0.5s
let echo2 = MidiNoteOn(60u, 60u) * 0.25; // Echo at 1.0s
```

#### Temporal Analysis
```wgsl
// Analyze activity over time
var total_activity = 0.0;
for (var i = 0u; i < 60u; i++) {         // Last second at 60fps
    total_activity += MidiNote(60u, i);
}
let average_activity = total_activity / 60.0;
```

### Performance Characteristics

#### Memory Usage
- **Total Size**: 768KB storage buffer (well within WebGPU limits)
- **Frame Update**: Ring buffer provides O(1) frame advancement
- **GPU Access**: Direct array indexing for historical data

#### Transfer Optimization
- **Full Transfer**: Complete history transferred each frame (768KB)
- **Future Optimization**: Differential updates could reduce transfer size
- **GPU Caching**: Historical data cached in GPU memory between frames