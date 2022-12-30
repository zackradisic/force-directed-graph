use std::collections::BTreeMap;

use bytemuck::{Pod, Zeroable};
use cgmath::{vec3, InnerSpace};
use wgpu::util::DeviceExt;

use crate::{node::Node, texture::Texture, Vertex, SAMPLE_COUNT};

pub const DEFAULT_INSTANCE_BUFFER_CAP: usize = 1024;

pub struct EdgeRenderPass {
    pub edges: Vec<Edge>,
    pub edge_map: BTreeMap<u32, Vec<u32>>,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
}

pub struct Edge {
    pub a_id: u32,
    pub b_id: u32,
    pub a_center: cgmath::Vector3<f32>,
    pub b_center: cgmath::Vector3<f32>,

    pub color: cgmath::Vector4<f32>,
    pub line_width: f32,
}

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct EdgeRaw {
    pub color: [f32; 4],
    pub a: [f32; 3],
    pub b: [f32; 3],
    pub a_norm: [f32; 3],
    pub b_norm: [f32; 3],
    pub line_width: f32,
}

impl EdgeRenderPass {
    const INDICES: &[u16] = &[0, 1, 3, 3, 1, 2];
    const VERTICES: &[Vertex] = &[
        Vertex {
            // vertex a, index = 0
            position: [-1.0, -1.0],
        },
        Vertex {
            // vertex b, index = 1
            position: [1.0, -1.0],
        },
        Vertex {
            // vertex c, index = 2
            position: [1.0, 1.0],
        },
        Vertex {
            // vertex d, index = 3
            position: [-1.0, 1.0],
        },
    ];

    pub fn new(
        edges: Vec<Edge>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let edges = edges;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Edge Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("edge.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Edge"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Edge Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), EdgeRaw::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    // blend: Some(wgpu::BlendState {
                    //     color: wgpu::BlendComponent {
                    //         src_factor: wgpu::BlendFactor::One,
                    //         dst_factor: wgpu::BlendFactor::One,
                    //         operation: wgpu::BlendOperation::
                    //     },
                    //     alpha: wgpu::BlendComponent::OVER,
                    // }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                unclipped_depth: false,
                ..Default::default()
            },
            // depth_stencil: None,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT as u32,
                ..Default::default()
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edge Vertex Buffer"),
            contents: bytemuck::cast_slice(Self::VERTICES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "Edge Index Buffer".into(),
            contents: bytemuck::cast_slice(Self::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Node Instance Buffer"),
            size: (std::mem::size_of::<EdgeRaw>() * DEFAULT_INSTANCE_BUFFER_CAP) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &instance_buffer,
            0,
            bytemuck::cast_slice(&edges.iter().map(Edge::to_instance).collect::<Vec<_>>()),
        );

        let mut node_to_edge = BTreeMap::new();

        for (i, edge) in edges.iter().enumerate() {
            node_to_edge
                .entry(edge.a_id)
                .or_insert_with(Vec::new)
                .push(i as u32);
            node_to_edge
                .entry(edge.b_id)
                .or_insert_with(Vec::new)
                .push(i as u32);
        }

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            edge_map: node_to_edge,
            edges,
        }
    }

    pub fn write(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.edges.iter().map(Edge::to_instance).collect::<Vec<_>>()),
        );
    }

    pub fn render<'a, 'b>(
        &'a self,
        camera_bind_group: &'a wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        if self.edges.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..Self::INDICES.len() as u32, 0, 0..self.edges.len() as u32);
    }
}

impl Edge {
    pub fn from_nodes(
        (a, a_id): (&Node, u32),
        (b, b_id): (&Node, u32),
        color: cgmath::Vector4<f32>,
        line_width: f32,
    ) -> Self {
        Self {
            a_id,
            b_id,
            a_center: a.position,
            b_center: b.position,
            color,
            line_width,
        }
    }

    pub fn to_instance(&self) -> EdgeRaw {
        let dx = self.b_center.x - self.a_center.x;
        let dy = self.b_center.y - self.a_center.y;

        let a_norm = vec3(-dy, dx, 0.0).normalize();
        let b_norm = vec3(dy, -dx, 0.0).normalize();

        EdgeRaw {
            color: self.color.into(),
            a: self.a_center.into(),
            b: self.b_center.into(),
            a_norm: a_norm.into(),
            b_norm: b_norm.into(),
            line_width: self.line_width,
        }
    }
}

impl EdgeRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x3,
        3 => Float32x3,
        4 => Float32x3,
        5 => Float32x3,
        6 => Float32,
    ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<EdgeRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
