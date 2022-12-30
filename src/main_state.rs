use cgmath::{vec2, vec3, Rotation3};
use winit::{event::WindowEvent, window::Window};

use crate::{
    camera::Camera,
    node::{Node, NodeRenderPass},
    screen_space_to_clip_space, screen_vec_to_clip_vec,
    texture::Texture,
    SAMPLE_COUNT,
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
    pub nodes: NodeRenderPass,
    // pub edges: EdgeRenderPass,
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
            vec![Node::new(
                vec2(50.0, 50.0),
                (0.0, 0.0, 0.0),
                cgmath::Quaternion::from_axis_angle(cgmath::vec3(0.0, 0.0, 0.0), cgmath::Deg(0.0)),
                (1.0, 1.0, 1.0, 1.0),
            )],
            &device,
            format,
            &camera_bind_group_layout,
        );

        Self {
            surface,
            device,
            queue,
            config,
            size,
            depth_texture,
            msaa_texture,
            camera,
            nodes,
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
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

    pub fn update(&mut self) {}

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

            self.nodes.render(&self.camera.bind_group, &mut render_pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
