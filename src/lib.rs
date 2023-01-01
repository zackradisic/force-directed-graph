pub mod camera;
pub mod edge;
pub mod input;
pub mod main_state;
pub mod mouse;
pub mod node;
pub mod physics;
pub mod texture;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use std::time::{SystemTime, UNIX_EPOCH};

use bytemuck::{Pod, Zeroable};
use cgmath::Vector4;
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

// For MacOS bc retina screens double the amount of pixels
pub const SCREEN_SCALE: f32 = 2.0;

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

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    let mut frame: u128 = 0;
    let mut start: u128 = 0;

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

    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        // window.set_inner_size(PhysicalSize::new(1600, 1200));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut state = pollster::block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| {
        if start == 0 {
            start = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis();
        }

        match event {
            Event::DeviceEvent { event, .. } => {
                state.device_input(&event);
            }
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
                frame += 1;
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                if now.as_millis() - start > 1000 {
                    let fps = frame as f64 / ((now.as_millis() - start) as f64 / 1000.0);
                    // window.set_title(&format!("{:.1$} fps", fps, 3));
                    window.set_title(&format!(
                        "{} fps — Nodes {}",
                        fps,
                        state.node_render_pass.nodes.len()
                    ));
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

pub struct ColorGenerator {
    pub colors: Vec<Vector4<f32>>,
    pub idx: usize,
}

impl ColorGenerator {
    pub fn new() -> Self {
        Self {
            colors: vec![
                Self::hex_to_rgba("5FB49C"),
                Self::hex_to_rgba("F2B134"),
                Self::hex_to_rgba("F93943"),
                Self::hex_to_rgba("6EF9F5"),
                Self::hex_to_rgba("B33C86"),
                Self::hex_to_rgba("E4FF1A"),
                Self::hex_to_rgba("FFB800"),
                Self::hex_to_rgba("FF5714"),
                Self::hex_to_rgba("FFEECF"),
                Self::hex_to_rgba("4D9078"),
                Self::hex_to_rgba("D5F2E3"),
                Self::hex_to_rgba("FBF5F3"),
                Self::hex_to_rgba("C6CAED"),
                Self::hex_to_rgba("A288E3"),
                Self::hex_to_rgba("CCFFCB"),
            ],
            idx: 0,
        }
    }

    pub fn next(&mut self) -> Vector4<f32> {
        let idx = self.idx % self.colors.len();
        self.idx += 1;
        self.colors[idx].clone()
    }

    fn hex_to_rgba(hex: &str) -> Vector4<f32> {
        let mut hex = hex.to_string();
        if hex.len() == 3 {
            hex = format!(
                "{}{}{}{}{}{}",
                hex.chars().nth(0).unwrap(),
                hex.chars().nth(0).unwrap(),
                hex.chars().nth(1).unwrap(),
                hex.chars().nth(1).unwrap(),
                hex.chars().nth(2).unwrap(),
                hex.chars().nth(2).unwrap()
            );
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap();
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap();
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap();
        // let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
        Vector4::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    }
}

impl Iterator for ColorGenerator {
    type Item = Vector4<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next())
    }
}
