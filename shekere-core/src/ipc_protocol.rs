use serde::{Deserialize, Serialize};

/// IPC Protocol for communication between Tauri backend and WASM frontend
/// This defines the data structures for real-time uniform data transfer

/// Main IPC message envelope that wraps all communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum IpcMessage {
    /// Uniform data updates from backend to frontend
    UniformUpdate(UniformData),
    /// Configuration updates (shader loading, hot reload)
    ConfigUpdate(ConfigData),
    /// Error messages
    Error(ErrorData),
    /// Heartbeat/ping messages for connection health
    Heartbeat,
    /// Initialization complete signal
    InitComplete,
}

/// Uniform data that needs to be transferred from backend to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniformData {
    /// Current time in seconds since start
    pub time: f32,
    /// Delta time since last frame
    pub delta_time: f32,
    /// Frame count since start
    pub frame: u32,
    /// Mouse position (normalized 0-1)
    pub mouse: Option<MouseData>,
    /// Audio spectrum analysis data
    pub spectrum: Option<SpectrumData>,
    /// MIDI input data
    pub midi: Option<MidiData>,
    /// OSC input data
    pub osc: Option<OscData>,
    /// Resolution (width, height)
    pub resolution: [f32; 2],
}

/// Mouse input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseData {
    /// Current mouse position (normalized 0-1)
    pub position: [f32; 2],
    /// Mouse click state (left, right, middle buttons)
    pub buttons: [bool; 3],
    /// Mouse wheel delta
    pub wheel: f32,
}

/// Audio spectrum analysis data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumData {
    /// Frequency bins (typically 512 or 1024 values)
    pub frequencies: Vec<f32>,
    /// Overall amplitude/volume
    pub amplitude: f32,
    /// Frequency of the peak
    pub peak_frequency: f32,
    /// Sample rate used for analysis
    pub sample_rate: u32,
}

/// MIDI input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiData {
    /// Active MIDI notes (note number, velocity)
    pub active_notes: Vec<(u8, u8)>,
    /// Control changes (controller number, value)
    pub control_changes: Vec<(u8, u8)>,
    /// Program change
    pub program_change: Option<u8>,
    /// Pitch bend value (-8192 to 8191)
    pub pitch_bend: i16,
}

/// OSC (Open Sound Control) input data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscData {
    /// OSC messages received this frame
    pub messages: Vec<OscMessage>,
}

/// Individual OSC message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OscMessage {
    /// OSC address pattern
    pub address: String,
    /// OSC arguments
    pub args: Vec<OscArg>,
}

/// OSC argument types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum OscArg {
    Float(f32),
    Int(i32),
    String(String),
    Bool(bool),
    Blob(Vec<u8>),
}

/// Configuration data for shader loading and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    /// Shader configuration update
    pub shader_config: Option<ShaderConfigData>,
    /// Hot reload event
    pub hot_reload: Option<HotReloadData>,
    /// Error in configuration
    pub error: Option<String>,
}

/// Shader configuration data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderConfigData {
    /// Shader source code
    pub shader_source: String,
    /// Entry point function name
    pub entry_point: String,
    /// Shader type (fragment, compute, etc.)
    pub shader_type: ShaderType,
    /// Pipeline label/name
    pub label: String,
}

/// Shader types supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShaderType {
    Fragment,
    Compute,
    Vertex,
}

/// Hot reload event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotReloadData {
    /// Path to the file that changed
    pub file_path: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Timestamp of the change
    pub timestamp: u64,
}

/// Types of file changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Modified,
    Created,
    Deleted,
    Renamed { from: String, to: String },
}

/// Error data for IPC communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorData {
    /// Error message
    pub message: String,
    /// Error code (optional)
    pub code: Option<u32>,
    /// Context information
    pub context: Option<String>,
}

/// IPC communication settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcSettings {
    /// Maximum update frequency (Hz)
    pub max_update_rate: f32,
    /// Enable compression for large data transfers
    pub compression_enabled: bool,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for IpcSettings {
    fn default() -> Self {
        Self {
            max_update_rate: 60.0, // 60 FPS
            compression_enabled: false, // Keep simple for Phase 1
            max_message_size: 1024 * 1024, // 1MB max
        }
    }
}

/// Utility functions for IPC protocol
impl IpcMessage {
    /// Serialize the message to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Check if this is a high-frequency update that should be throttled
    pub fn is_high_frequency_update(&self) -> bool {
        matches!(self, IpcMessage::UniformUpdate(_))
    }

    /// Get the priority of this message (higher number = higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            IpcMessage::Error(_) => 255,
            IpcMessage::ConfigUpdate(_) => 200,
            IpcMessage::InitComplete => 150,
            IpcMessage::UniformUpdate(_) => 100,
            IpcMessage::Heartbeat => 50,
        }
    }
}

/// Helper for creating uniform updates
impl UniformData {
    pub fn new(time: f32, delta_time: f32, frame: u32, resolution: [f32; 2]) -> Self {
        Self {
            time,
            delta_time,
            frame,
            mouse: None,
            spectrum: None,
            midi: None,
            osc: None,
            resolution,
        }
    }

    /// Create a minimal update with just timing information
    pub fn minimal(time: f32, delta_time: f32, frame: u32) -> Self {
        Self::new(time, delta_time, frame, [800.0, 600.0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_serialization() {
        let uniform_data = UniformData::minimal(1.0, 0.016, 60);
        let message = IpcMessage::UniformUpdate(uniform_data);

        let json = message.to_json().expect("Failed to serialize");
        let deserialized = IpcMessage::from_json(&json).expect("Failed to deserialize");

        match deserialized {
            IpcMessage::UniformUpdate(data) => {
                assert_eq!(data.time, 1.0);
                assert_eq!(data.delta_time, 0.016);
                assert_eq!(data.frame, 60);
            }
            _ => panic!("Wrong message type deserialized"),
        }
    }

    #[test]
    fn test_message_priority() {
        let error_msg = IpcMessage::Error(ErrorData {
            message: "Test error".to_string(),
            code: None,
            context: None,
        });
        let uniform_msg = IpcMessage::UniformUpdate(UniformData::minimal(0.0, 0.0, 0));
        let heartbeat_msg = IpcMessage::Heartbeat;

        assert!(error_msg.priority() > uniform_msg.priority());
        assert!(uniform_msg.priority() > heartbeat_msg.priority());
    }

    #[test]
    fn test_high_frequency_detection() {
        let uniform_msg = IpcMessage::UniformUpdate(UniformData::minimal(0.0, 0.0, 0));
        let error_msg = IpcMessage::Error(ErrorData {
            message: "Test".to_string(),
            code: None,
            context: None,
        });

        assert!(uniform_msg.is_high_frequency_update());
        assert!(!error_msg.is_high_frequency_update());
    }
}