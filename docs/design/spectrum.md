# Spectrum Analysis Specification

## Overview

Spectrum analysis provides real-time FFT (Fast Fourier Transform) analysis of system audio input. This system enables audio-reactive visual effects based on frequency domain analysis.

## Purpose

Real-time FFT analysis of system audio input for frequency-based visual effects and music visualization.

## Features

- **FFT-based Analysis**: Frequency domain analysis of audio input
- **Configurable Frequency Range**: Set minimum and maximum frequency bounds
- **Configurable Sampling Rate**: Adjust sampling rate for analysis precision
- **System Audio Input**: Process audio from system audio input
- **Real-time Processing**: Continuous spectrum data for visual effects

## Configuration

```toml
[spectrum]
min_frequency = 20.0
max_frequency = 20000.0
sampling_rate = 44100
```

### Configuration Parameters

- `min_frequency`: Minimum frequency (Hz) for analysis range
- `max_frequency`: Maximum frequency (Hz) for analysis range  
- `sampling_rate`: Audio sampling rate for FFT analysis

## GPU Integration

### Uniform Data Creation
- Spectrum data is processed and converted to GPU-accessible uniform buffers
- Data is made available to shaders through binding group 2 (Sound Uniforms)
- Real-time updates ensure smooth audio-visual synchronization

### vec4 Packing Optimization
- All spectrum uniforms use vec4 packing for WebGPU alignment
- Ensures optimal memory layout for GPU processing
- Improves performance by reducing memory bandwidth requirements

## Shader Integration

### Sound Uniforms (Binding Group 2)
When spectrum analysis is configured, shaders can access:
- **Spectrum Data**: Frequency domain analysis results
- Amplitude data for specific frequency ranges

### Helper Functions
Shaders can use helper functions for spectrum data access. For detailed function signatures and usage, see [API Reference](api-reference.md).

## Real-Time Processing

### Performance Considerations
- Audio processing runs on separate threads to avoid blocking rendering
- Efficient FFT algorithms for real-time analysis
- Optimized data structures for frequency domain processing
- Minimal latency between audio input and visual output

### Synchronization
- Spectrum data is synchronized with the render loop
- Thread-safe data structures for concurrent access
- Consistent frame timing for smooth audio-visual effects

## Audio Processing

### FFT Analysis
- Uses Fast Fourier Transform for frequency domain conversion
- Configurable window size and overlap for analysis precision
- Handles audio input buffering and processing

### Frequency Range Processing
- Filters analysis results to configured frequency range
- Maps frequency bins to uniform array indices
- Provides amplitude data for specified frequency bands

### System Audio Input
- Captures audio from system audio input devices
- Handles audio device selection and management
- Supports various audio formats and sample rates