// Mouse input definitions for shekere shaders

// === MOUSE STRUCTURES ===

struct MouseShaderData {
    // mouse position (vec2 with vec4 alignment)
    position: vec2<f32>,
    _padding: vec2<f32>, // vec4 alignment for GPU efficiency
}

struct MouseHistory {
    // 512 frames of mouse history data (8KB total)
    // Index 0 = current frame, Index 511 = oldest frame
    history_data: array<MouseShaderData, 512>,
}

// === MOUSE BINDINGS ===

// Group 2: Mouse storage buffer
@group(2) @binding(2) var<storage, read> Mouse: MouseHistory;

// === MOUSE FUNCTIONS ===

fn MouseCoordsHistory(history: u32) -> vec2<f32> {
    if history >= 512u {
        return vec2<f32>(0.0, 0.0);
    }

    let mouse_data = Mouse.history_data[history];
    // Convert to normalized coordinates
    let min_xy = min(Window.resolution.x, Window.resolution.y);
    return (mouse_data.position * 2.0 - Window.resolution) / min_xy;
}

fn MouseCoords() -> vec2<f32> {
    return MouseCoordsHistory(0u);
}