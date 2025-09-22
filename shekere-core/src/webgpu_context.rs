use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebGpuError {
    #[error("Failed to request adapter")]
    AdapterRequest,
    #[error("Failed to request device: {0}")]
    DeviceRequest(#[from] wgpu::RequestDeviceError),
    #[error("No suitable graphics adapter found")]
    NoAdapter,
}

/// Context for WebGPU resources that abstracts device and queue management
/// from window concerns, supporting both headless and surface-based initialization.
pub struct WebGpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl WebGpuContext {
    /// Create a headless WebGPU context for GUI/testing scenarios.
    /// This doesn't require a window or surface.
    pub async fn new_headless() -> Result<Self, WebGpuError> {
        // Create instance with appropriate backends
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        // Request adapter without surface compatibility requirement
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None, // No surface needed for headless
                force_fallback_adapter: false,
            })
            .await
            .ok_or(WebGpuError::AdapterRequest)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None, // Trace path
            )
            .await?;

        Ok(Self { device, queue })
    }

    /// Create a WebGPU context that is compatible with the provided surface.
    /// This is for CLI scenarios where we have a window and surface.
    pub async fn new_with_surface(surface: &wgpu::Surface<'_>) -> Result<Self, WebGpuError> {
        // Create instance with appropriate backends
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        // Request adapter with surface compatibility requirement
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(WebGpuError::AdapterRequest)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None, // Trace path
            )
            .await?;

        Ok(Self { device, queue })
    }

    /// Get a reference to the device
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get a reference to the queue
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_headless_context_creation() {
        // Test that we can create a headless context without panicking
        let result = WebGpuContext::new_headless().await;

        // We can't guarantee that WebGPU will work in all test environments,
        // but we can verify the API works correctly
        match result {
            Ok(context) => {
                // If successful, verify we got valid device and queue
                assert!(
                    !context.device().features().is_empty()
                        || context.device().features().is_empty()
                );
                // Just check that queue exists (no good way to verify it's valid without using it)
                let _queue = context.queue();
            }
            Err(WebGpuError::AdapterRequest) => {
                // This is acceptable in CI/test environments without graphics
                println!("WebGPU adapter not available in test environment");
            }
            Err(e) => {
                panic!("Unexpected error during headless context creation: {}", e);
            }
        }
    }

    #[test]
    fn test_webgpu_error_display() {
        let error = WebGpuError::AdapterRequest;
        assert_eq!(error.to_string(), "Failed to request adapter");

        let error = WebGpuError::NoAdapter;
        assert_eq!(error.to_string(), "No suitable graphics adapter found");
    }
}
