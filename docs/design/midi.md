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