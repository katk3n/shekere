// MIDI input definitions for shekere shaders

// === MIDI STRUCTURES ===

struct MidiShaderData {
    // note velocities (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    notes: array<vec4<f32>, 32>,
    // control change values (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    controls: array<vec4<f32>, 32>,
    // note on attack detection (0-127 normalized to 0.0-1.0)
    // Packed into vec4s for alignment: 128 values in 32 vec4s
    note_on: array<vec4<f32>, 32>,
}

struct MidiHistory {
    // 512 frames of MIDI history data (768KB total)
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<MidiShaderData, 512>,
}

// === MIDI BINDINGS ===

// Group 2: MIDI storage buffer
@group(2) @binding(5) var<storage, read> Midi: MidiHistory;

// === MIDI FUNCTIONS ===

// MIDI helper functions
fn MidiNote(note_num: u32) -> f32 {
    return MidiNoteHistory(note_num, 0u);
}

fn MidiControl(cc_num: u32) -> f32 {
    return MidiControlHistory(cc_num, 0u);
}

fn MidiNoteOn(note_num: u32) -> f32 {
    return MidiNoteOnHistory(note_num, 0u);
}

// MIDI History helper functions
fn MidiNoteHistory(note_num: u32, history: u32) -> f32 {
    let vec4_index = note_num / 4u;
    let element_index = note_num % 4u;

    // Bounds checking
    if vec4_index >= 32u || history >= 512u {
        return 0.0;
    }

    let note_vec = Midi.history_data[history].notes[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}

fn MidiControlHistory(cc_num: u32, history: u32) -> f32 {
    let vec4_index = cc_num / 4u;
    let element_index = cc_num % 4u;

    // Bounds checking
    if vec4_index >= 32u || history >= 512u {
        return 0.0;
    }

    let cc_vec = Midi.history_data[history].controls[vec4_index];
    switch element_index {
        case 0u: { return cc_vec.x; }
        case 1u: { return cc_vec.y; }
        case 2u: { return cc_vec.z; }
        case 3u: { return cc_vec.w; }
        default: { return 0.0; }
    }
}

fn MidiNoteOnHistory(note_num: u32, history: u32) -> f32 {
    let vec4_index = note_num / 4u;
    let element_index = note_num % 4u;

    // Bounds checking
    if vec4_index >= 32u || history >= 512u {
        return 0.0;
    }

    let note_vec = Midi.history_data[history].note_on[vec4_index];
    switch element_index {
        case 0u: { return note_vec.x; }
        case 1u: { return note_vec.y; }
        case 2u: { return note_vec.z; }
        case 3u: { return note_vec.w; }
        default: { return 0.0; }
    }
}