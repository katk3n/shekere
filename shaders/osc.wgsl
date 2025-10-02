// OSC input definitions for shekere shaders

// === OSC STRUCTURES ===

struct OscShaderData {
    // OSC sound values (packed into vec4s for alignment)
    sounds: array<vec4<i32>, 4>,
    // OSC ttl values (packed into vec4s for alignment)
    ttls: array<vec4<f32>, 4>,
    // OSC note values (packed into vec4s for alignment)
    notes: array<vec4<f32>, 4>,
    // OSC gain values (packed into vec4s for alignment)
    gains: array<vec4<f32>, 4>,
}

struct OscHistory {
    // 512 frames of OSC history data
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<OscShaderData, 512>,
}

// === OSC BINDINGS ===

// Group 2: OSC storage buffer
@group(2) @binding(4) var<storage, read> Osc: OscHistory;

// === OSC FUNCTIONS ===

// OSC data access helpers
fn OscSoundHistory(index: u32, history: u32) -> i32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;

    // Bounds checking
    if vec4_index >= 4u || history >= 512u {
        return 0;
    }

    let sound_vec = Osc.history_data[history].sounds[vec4_index];
    switch element_index {
        case 0u: { return sound_vec.x; }
        case 1u: { return sound_vec.y; }
        case 2u: { return sound_vec.z; }
        case 3u: { return sound_vec.w; }
        default: { return 0; }
    }
}

fn OscTtlHistory(index: u32, history: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;

    // Bounds checking
    if vec4_index >= 4u || history >= 512u {
        return 0.0;
    }

    let ttl_vec = Osc.history_data[history].ttls[vec4_index];
    switch element_index {
        case 0u: { return ttl_vec.x; }
        case 1u: { return ttl_vec.y; }
        case 2u: { return ttl_vec.z; }
        case 3u: { return ttl_vec.w; }
        default: { return 0.0; }
    }
}

fn OscNoteHistory(index: u32, history: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;

    // Bounds checking
    if vec4_index >= 4u || history >= 512u {
        return 0.0;
    }

    let note_vec = Osc.history_data[history].notes[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}

fn OscGainHistory(index: u32, history: u32) -> f32 {
    let vec4_index = index / 4u;
    let element_index = index % 4u;

    // Bounds checking
    if vec4_index >= 4u || history >= 512u {
        return 0.0;
    }

    let gain_vec = Osc.history_data[history].gains[vec4_index];
    switch element_index {
        case 0u: { return gain_vec.x; }
        case 1u: { return gain_vec.y; }
        case 2u: { return gain_vec.z; }
        case 3u: { return gain_vec.w; }
        default: { return 0.0; }
    }
}

fn OscSound(index: u32) -> i32 {
    return OscSoundHistory(index, 0u);
}

fn OscTtl(index: u32) -> f32 {
    return OscTtlHistory(index, 0u);
}

fn OscNote(index: u32) -> f32 {
    return OscNoteHistory(index, 0u);
}

fn OscGain(index: u32) -> f32 {
    return OscGainHistory(index, 0u);
}