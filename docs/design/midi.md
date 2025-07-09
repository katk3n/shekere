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

### Device Management
- Automatic detection of connected MIDI devices
- Handle device connection/disconnection gracefully
- Support for multiple MIDI input devices