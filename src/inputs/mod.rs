pub mod midi;
pub mod mouse;
pub mod osc;
pub mod spectrum;

// Re-export Bevy-compatible types for convenience
pub use midi::{MidiInputManager, midi_input_system, setup_midi_input_system};
pub use mouse::{MouseInputManager, mouse_input_system, setup_mouse_input_system};
pub use osc::{OscInputManager, osc_input_system, setup_osc_input_system};
pub use spectrum::{SpectrumInputManager, setup_spectrum_input_system, spectrum_input_system};
