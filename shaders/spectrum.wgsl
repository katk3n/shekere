// Spectrum analysis definitions for shekere shaders

// === SPECTRUM STRUCTURES ===

struct SpectrumShaderData {
    // frequency values of audio input (packed into vec4s for alignment)
    frequencies: array<vec4<f32>, 512>,
    // amplitude values of audio input (packed into vec4s for alignment)
    amplitudes: array<vec4<f32>, 512>,
    // the number of data points
    num_points: u32,
    // frequency of the data point with the max amplitude
    max_frequency: f32,
    // max amplitude of audio input
    max_amplitude: f32,
    _padding: u32,
}

struct SpectrumHistory {
    // 512 frames of spectrum history data
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<SpectrumShaderData, 512>,
}

// === SPECTRUM BINDINGS ===

// Group 2: Spectrum storage buffer
@group(2) @binding(3) var<storage, read> Spectrum: SpectrumHistory;

// === SPECTRUM FUNCTIONS ===

// Spectrum data access helpers
fn SpectrumFrequencyHistory(index: u32, history: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;

    // Bounds checking
    if vec4_index >= 512u || history >= 512u {
        return 0.0;
    }

    let freq_vec = Spectrum.history_data[history].frequencies[vec4_index];
    switch element_index {
        case 0u: { return freq_vec.x; }
        case 1u: { return freq_vec.y; }
        case 2u: { return freq_vec.z; }
        case 3u: { return freq_vec.w; }
        default: { return 0.0; }
    }
}

fn SpectrumAmplitudeHistory(index: u32, history: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;

    // Bounds checking
    if vec4_index >= 512u || history >= 512u {
        return 0.0;
    }

    let amp_vec = Spectrum.history_data[history].amplitudes[vec4_index];
    switch element_index {
        case 0u: { return amp_vec.x; }
        case 1u: { return amp_vec.y; }
        case 2u: { return amp_vec.z; }
        case 3u: { return amp_vec.w; }
        default: { return 0.0; }
    }
}

fn SpectrumFrequency(index: u32) -> f32 {
    return SpectrumFrequencyHistory(index, 0u);
}

fn SpectrumAmplitude(index: u32) -> f32 {
    return SpectrumAmplitudeHistory(index, 0u);
}

// Helper functions for spectrum metadata access
fn SpectrumNumPoints() -> u32 {
    return Spectrum.history_data[0].num_points;
}

fn SpectrumMaxFrequency() -> f32 {
    return Spectrum.history_data[0].max_frequency;
}

fn SpectrumMaxAmplitude() -> f32 {
    return Spectrum.history_data[0].max_amplitude;
}