mod audio_stream;
mod bind_group_factory;
pub mod config;
pub mod hot_reload;
mod osc;
pub mod pipeline;
pub mod render_constants;
mod shader_preprocessor;
mod state;
pub mod texture_manager;
mod timer;
mod uniforms;
mod vertex;

pub use crate::config::Config;
pub use crate::config::ShaderConfig;
pub use crate::state::State;
use std::path::Path;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

pub async fn run(conf: &Config, conf_dir: &Path) {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("shekere")
        .with_inner_size(LogicalSize::new(conf.window.width, conf.window.height))
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(&window, conf, conf_dir)
        .await
        .expect("Failed to create state");

    let _ = event_loop.run(move |event, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window().id() => {
            if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => control_flow.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        state.window().request_redraw();

                        state.update();
                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                state.resize(*state.size())
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("OutOfMemory");
                                control_flow.exit();
                            }
                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Surface timeout")
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}
