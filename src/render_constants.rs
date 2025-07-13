//! Render constants and enums to replace magic numbers throughout the rendering system.
//!
//! This module centralizes all rendering-related constants to improve code readability,
//! maintainability, and type safety. It eliminates magic numbers that were previously
//! scattered throughout the codebase.

/// Bind group indices for different types of uniform data
///
/// These constants define the binding indices used in shader bind groups,
/// corresponding to the layout specified in shaders and pipeline creation.
pub mod bind_group {
    /// Uniform buffer bind group (time, window, mouse, etc.)
    pub const UNIFORM: u32 = 0;

    /// Device-specific uniform bind group
    pub const DEVICE: u32 = 1;

    /// Sound/audio uniform bind group (OSC, MIDI, spectrum)
    pub const SOUND: u32 = 2;

    /// Texture bind group for multipass rendering
    pub const TEXTURE: u32 = 3;
}

/// Frame buffering constants for double-buffered rendering
///
/// These constants and functions handle the ping-pong buffer calculations
/// used in multipass rendering and state preservation.
pub mod frame_buffer {
    /// Number of frame buffers used in double buffering
    pub const BUFFER_COUNT: usize = 2;

    /// Get the current frame buffer index for writing
    ///
    /// # Arguments
    /// * `frame` - The current frame number
    ///
    /// # Returns
    /// The buffer index to write to (0 or 1)
    #[inline]
    pub const fn current_buffer_index(frame: u64) -> usize {
        (frame as usize) % BUFFER_COUNT
    }

    /// Get the previous frame buffer index for reading
    ///
    /// In double buffering, we read from the previous frame while writing to the current.
    /// This function calculates which buffer contains the previous frame's data.
    ///
    /// # Arguments
    /// * `frame` - The current frame number
    ///
    /// # Returns
    /// The buffer index to read from (0 or 1)
    #[inline]
    pub const fn previous_buffer_index(frame: u64) -> usize {
        ((frame + 1) as usize) % BUFFER_COUNT
    }
}

/// Render pass configuration constants
///
/// These constants define common render pass settings used throughout
/// the rendering pipeline.
pub mod render_pass {
    /// Default clear color for render passes
    ///
    /// Black is used as the default background color for most render passes
    /// to provide a neutral starting point for shader effects.
    pub const DEFAULT_CLEAR_COLOR: wgpu::Color = wgpu::Color::BLACK;

    /// Default render pass descriptor configuration
    #[derive(Debug, Clone)]
    pub struct Config {
        pub clear_color: wgpu::Color,
        pub label: Option<&'static str>,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                clear_color: DEFAULT_CLEAR_COLOR,
                label: Some("Main Render Pass"),
            }
        }
    }

    impl Config {
        /// Create a new render pass config with a custom clear color
        pub fn with_clear_color(clear_color: wgpu::Color) -> Self {
            Self {
                clear_color,
                label: Some("Custom Render Pass"),
            }
        }

        /// Create a new render pass config with a custom label
        pub fn with_label(label: &'static str) -> Self {
            Self {
                clear_color: DEFAULT_CLEAR_COLOR,
                label: Some(label),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_group_indices() {
        assert_eq!(bind_group::UNIFORM, 0);
        assert_eq!(bind_group::DEVICE, 1);
        assert_eq!(bind_group::SOUND, 2);
        assert_eq!(bind_group::TEXTURE, 3);
    }

    #[test]
    fn test_frame_buffer_calculations() {
        // Test current buffer index
        assert_eq!(frame_buffer::current_buffer_index(0u64), 0);
        assert_eq!(frame_buffer::current_buffer_index(1u64), 1);
        assert_eq!(frame_buffer::current_buffer_index(2u64), 0);
        assert_eq!(frame_buffer::current_buffer_index(3u64), 1);

        // Test previous buffer index
        assert_eq!(frame_buffer::previous_buffer_index(0u64), 1);
        assert_eq!(frame_buffer::previous_buffer_index(1u64), 0);
        assert_eq!(frame_buffer::previous_buffer_index(2u64), 1);
        assert_eq!(frame_buffer::previous_buffer_index(3u64), 0);
    }

    #[test]
    fn test_render_pass_config() {
        let default_config = render_pass::Config::default();
        assert_eq!(default_config.clear_color, wgpu::Color::BLACK);
        assert_eq!(default_config.label, Some("Main Render Pass"));

        let custom_color_config = render_pass::Config::with_clear_color(wgpu::Color::WHITE);
        assert_eq!(custom_color_config.clear_color, wgpu::Color::WHITE);

        let custom_label_config = render_pass::Config::with_label("Test Pass");
        assert_eq!(custom_label_config.label, Some("Test Pass"));
    }

    #[test]
    fn test_buffer_count_constant() {
        assert_eq!(frame_buffer::BUFFER_COUNT, 2);
    }
}
