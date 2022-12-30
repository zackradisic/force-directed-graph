pub mod camera;
pub mod edge;
pub mod main_state;
pub mod node;
pub mod texture;

use bytemuck::{Pod, Zeroable};
use main_state::State;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub const SAMPLE_COUNT: u8 = 4;
// pub const WIDTH: f32 = 800.0;
// pub const HEIGHT: f32 = 600.0;

// pub const SX: f32 = 1.0 / (WIDTH * 2.0);
// pub const SY: f32 = 1.0 / (HEIGHT * 2.0);
// pub const SX: f32 = 1.0 / (WIDTH);
// pub const SY: f32 = 1.0 / (HEIGHT);

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub fn screen_space_to_clip_space(
    width: f32,
    height: f32,
    pos: &cgmath::Vector2<f32>,
) -> cgmath::Vector2<f32> {
    // (0, 0) -> (1920, 1080)
    // (-960, -540) -> (960, 540)
    // (-1, -1) -> (1, 1)
    let pos = cgmath::vec2(pos.x - (width), -(pos.y - (height)));
    let pos = cgmath::vec2(pos.x / (width), pos.y / (height));
    pos
}

pub fn clip_space_to_screen_space(
    width: f32,
    height: f32,
    pos: &cgmath::Vector2<f32>,
) -> cgmath::Vector2<f32> {
    let pos = cgmath::vec2(pos.x * width, pos.y * height);
    let pos = cgmath::vec2(pos.x + width, height - pos.y);
    pos
}

pub fn screen_vec_to_clip_vec(
    width: f32,
    height: f32,
    pos: &cgmath::Vector2<f32>,
) -> cgmath::Vector2<f32> {
    let pos = cgmath::vec2((2.0 * pos.x) / width, (2.0 * pos.y) / height);
    pos
}
pub fn clip_vec_to_screen_vec(
    width: f32,
    height: f32,
    pos: &cgmath::Vector2<f32>,
) -> cgmath::Vector2<f32> {
    let pos = cgmath::vec2((pos.x / 2.0) * width, (pos.y / 2.0) * height);
    pos
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0=>Float32x2];
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize {
            width: 800,
            height: 600,
        })
        // .with_min_inner_size()
        // .with_max_inner_size(LogicalSize {
        //     width: 800,
        //     height: 600,
        // })
        .with_resizable(true)
        .build(&event_loop)
        .unwrap();

    let mut state = pollster::block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            _ => {}
        }
    });
}
