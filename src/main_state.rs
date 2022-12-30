use cgmath::{vec2, vec3, Rotation3, SquareMatrix};
use winit::{
    event::{DeviceEvent, ElementState, WindowEvent},
    window::Window,
};

use crate::{
    camera::Camera,
    edge::{Edge, EdgeRenderPass},
    input::InputState,
    mouse::Mouse,
    node::{Node, NodeRenderPass},
    physics::{self, Physics, DEFAULT_STRENGTH},
    screen_space_to_clip_space, screen_vec_to_clip_vec,
    texture::Texture,
    SAMPLE_COUNT, SCREEN_SCALE,
};

pub struct State {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub depth_texture: Texture,
    pub msaa_texture: Texture,

    pub camera: Camera,
    pub node_render_pass: NodeRenderPass,
    pub edge_render_pass: EdgeRenderPass,
    pub physics: Physics,
    pub mouse: Mouse,
    pub input: InputState, // pub edges: EdgeRenderPass,
}

impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::DEPTH_CLIP_CONTROL,
                    limits: wgpu::Limits::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let format = surface.get_supported_formats(&adapter)[3];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
        };
        surface.configure(&device, &config);

        let (w, h) = (800.0, 600.0);
        let (camera, camera_bind_group_layout) =
            Camera::new(cgmath::vec3(0.0, 0.0, 1.0), w, h, 1.0, &device);

        let depth_texture = Texture::create_depth_texture(&device, &config, SAMPLE_COUNT, "Depth");
        let msaa_texture = Texture::create_msaa_texture(&device, &config, "MSAA", SAMPLE_COUNT);

        let nodes = NodeRenderPass::new(
            vec![
                Node::new(
                    vec2(50.0, 50.0),
                    (0.0, 0.0, 0.0),
                    cgmath::Quaternion::from_axis_angle(
                        cgmath::vec3(0.0, 0.0, 0.0),
                        cgmath::Deg(0.0),
                    ),
                    (1.0, 1.0, 1.0, 1.0),
                ),
                Node::new(
                    vec2(50.0, 50.0),
                    (-50.0 * 2.0, 0.0, 0.0),
                    cgmath::Quaternion::from_axis_angle(
                        cgmath::vec3(0.0, 0.0, 0.0),
                        cgmath::Deg(0.0),
                    ),
                    (1.0, 0.0, 1.0, 1.0),
                ),
                Node::new(
                    vec2(50.0, 50.0),
                    (50.0 * 2.0, 0.0, 0.0),
                    cgmath::Quaternion::from_axis_angle(
                        cgmath::vec3(0.0, 0.0, 0.0),
                        cgmath::Deg(0.0),
                    ),
                    (1.0, 0.0, 0.0, 1.0),
                ),
                Node::new(
                    vec2(50.0, 50.0),
                    (50.0 * 2.0, -50.0 * 2.0, 0.0),
                    cgmath::Quaternion::from_axis_angle(
                        cgmath::vec3(0.0, 0.0, 0.0),
                        cgmath::Deg(0.0),
                    ),
                    (0.0, 1.0, 0.0, 1.0),
                ),
            ],
            &device,
            &queue,
            format,
            &camera_bind_group_layout,
        );

        let edges = EdgeRenderPass::new(
            vec![
                Edge::from_nodes(
                    (&nodes.nodes[0], 0),
                    (&nodes.nodes[1], 1),
                    cgmath::vec4(0.0, 1.0, 0.0, 1.0),
                    10.0,
                ),
                Edge::from_nodes(
                    (&nodes.nodes[1], 1),
                    (&nodes.nodes[2], 2),
                    cgmath::vec4(0.0, 1.0, 0.0, 1.0),
                    10.0,
                ),
                Edge::from_nodes(
                    (&nodes.nodes[0], 1),
                    (&nodes.nodes[2], 2),
                    cgmath::vec4(0.0, 1.0, 0.0, 1.0),
                    10.0,
                ),
            ],
            &device,
            &queue,
            format,
            &camera_bind_group_layout,
        );

        let physics = Physics::new(&nodes.nodes);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
            msaa_texture,
            camera,
            node_render_pass: nodes,
            edge_render_pass: edges,
            physics,
            mouse: Mouse::default(),
            input: InputState::default(),
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseWheel { delta, phase, .. } => {
                let y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y as f64,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => pos.y / 100.,
                };

                self.camera
                    .update_scale(&self.queue, self.camera.scale + y as f32);
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse.last_pos = self.mouse.pos.unwrap_or((0.0, 0.0).into());
                self.mouse.pos = None;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let mut vec: cgmath::Vector2<f32> = (position.x as f32, position.y as f32).into();
                vec.x -= self.camera.width / 2.0;
                vec.y -= self.camera.height / 2.0;
                // vec.x *= 2.0;
                vec.y *= -1.0;
                println!("CURSOR: {:?}", vec);
                self.mouse.pos = Some(vec);
            }
            _ => (),
        }
        false
    }

    pub fn add_node(&mut self, node: Node) {
        let idx = self.node_render_pass.nodes.len();
        self.physics.objs.push(physics::Object::from_node(
            idx as u32,
            &node,
            DEFAULT_STRENGTH,
        ));
        self.node_render_pass.add_node(node, &self.queue)
    }

    pub fn set_dragging(&mut self, dragging: Option<u32>) {
        self.input.dragging = dragging;
    }

    pub fn device_input(&mut self, event: &DeviceEvent) -> bool {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if let Some(node) = self.input.dragging {
                    self.node_render_pass.nodes[node as usize].position.x +=
                        delta.0 as f32 * 2.0 * (1. / self.camera.scale);
                    self.node_render_pass.nodes[node as usize].position.y += -delta.1 as f32
                        * 2.0
                        // * (self.camera.height / self.camera.width)
                        * (1. / self.camera.scale);
                    self.node_render_pass.update_node(node, &self.queue);
                    self.physics.objs[node as usize].x =
                        self.node_render_pass.nodes[node as usize].position.x;
                    self.physics.objs[node as usize].y =
                        self.node_render_pass.nodes[node as usize].position.y;
                }
            }
            DeviceEvent::Button { state, .. } => match state {
                ElementState::Pressed => {
                    if let Some(pos) = &self.mouse.pos {
                        let pos = pos / self.camera.scale;

                        // let clip_pos =
                        //     screen_space_to_clip_space(self.camera.width, self.camera.height, pos);
                        // let clip_pos =
                        //     self.camera.matrix.invert().unwrap() * clip_pos.extend(0.0).extend(1.0);
                        let pos3 = pos.extend(0.0);
                        if let Some((i, _)) = self
                            .node_render_pass
                            .nodes
                            .iter()
                            .enumerate()
                            .find(|(_, node)| node.intersects(&pos3))
                        {
                            self.set_dragging(Some(i as u32));
                            return false;
                        }

                        let node = Node::new(
                            (50.0, 50.0),
                            pos.extend(0.0),
                            cgmath::Quaternion::from_axis_angle(
                                cgmath::vec3(0.0, 0.0, 0.0),
                                cgmath::Deg(0.0),
                            ),
                            (0.0, 1.0, 1.0, 1.0),
                        );
                        self.add_node(node);
                    }
                }
                ElementState::Released => {
                    self.set_dragging(None);
                }
            },
            _ => (),
        }
        false
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                Texture::create_depth_texture(&self.device, &self.config, SAMPLE_COUNT, "depth");
            self.camera
                .resize(new_size.width as f32, new_size.height as f32, &self.queue);
        }
    }

    pub fn update(&mut self) {
        self.physics.tick(
            self.input.dragging,
            &self.edge_render_pass.edges,
            &self.edge_render_pass.edge_map,
        );
        self.physics.apply(
            self.node_render_pass.nodes.as_mut_slice(),
            &mut self.edge_render_pass.edges,
            &self.edge_render_pass.edge_map,
        );
        self.node_render_pass.write(&self.queue);
        self.edge_render_pass.write(&self.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let (view, resolve_target) = if SAMPLE_COUNT > 1 {
                (&self.msaa_texture.view, Some(&view))
            } else {
                (&view, None)
            };

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 20.0 / 256.0,
                            g: 20.0 / 256.,
                            b: 28.0 / 256.,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                // depth_stencil_attachment: None,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            self.edge_render_pass
                .render(&self.camera.bind_group, &mut render_pass);
            self.node_render_pass
                .render(&self.camera.bind_group, &mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
