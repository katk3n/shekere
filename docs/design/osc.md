# OSC Integration Specification

## Overview

OSC (Open Sound Control) integration enables real-time audio control through network messages. This system is designed for live coding performances and integration with external audio software.

## Purpose

Receives Open Sound Control (OSC) messages for real-time audio control and parameter manipulation.

## Features

- **Configurable Port**: Set custom port for OSC message reception
- **Address Pattern Matching**: Filter OSC messages by address pattern
- **Sound Mapping**: Configure sound names and IDs for parameter control
- **Tidalcycles Integration**: Seamless integration with Tidalcycles for live coding performances
- **Real-time Updates**: Immediate parameter updates from external audio software

## Configuration

```toml
[osc]
port = 2020
addr_pattern = "/dirt/play"

[[osc.sound]]
name = "bd"
id = 1

[[osc.sound]]
name = "sd"
id = 2
```

### Configuration Parameters

- `port`: UDP port number for OSC message reception
- `addr_pattern`: OSC address pattern to match incoming messages
- `sound[]`: Array of sound configurations
  - `name`: Sound identifier name
  - `id`: Numeric ID for shader uniform access

## GPU Integration

### Uniform Data Creation
- OSC data is processed and converted to GPU-accessible uniform buffers
- Data is made available to shaders through binding group 2 (Sound Uniforms)
- Real-time updates ensure smooth audio-visual synchronization

### vec4 Packing Optimization
- All OSC uniforms use vec4 packing for WebGPU alignment
- Ensures optimal memory layout for GPU processing
- Improves performance by reducing memory bandwidth requirements

## Shader Integration

### Sound Uniforms (Binding Group 2)
When OSC is configured, shaders can access:
- **OSC Data**: Real-time OSC parameter values
- Sound-specific parameters mapped by ID

### Helper Functions
Shaders can use helper functions for OSC data access. For detailed function signatures and usage, see [API Reference](api-reference.md).

## Real-Time Processing

### Performance Considerations
- OSC processing runs on separate threads to avoid blocking rendering
- Efficient data structures for real-time message processing
- Minimal latency between OSC input and visual output

### Synchronization
- OSC data is synchronized with the render loop
- Thread-safe data structures for concurrent access
- Consistent frame timing for smooth audio-visual effects

## Network Protocol

### UDP Communication
- Uses UDP protocol for low-latency message transmission
- Handles message parsing and validation
- Supports standard OSC message format

### Message Processing
- Incoming messages are filtered by address pattern
- Sound parameters are extracted and mapped to uniform data
- Invalid messages are discarded gracefully