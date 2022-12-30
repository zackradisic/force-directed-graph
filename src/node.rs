use std::mem::MaybeUninit;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::{texture::Texture, Vertex, SAMPLE_COUNT};

pub const DEFAULT_INSTANCE_BUFFER_CAP: usize = 1024;

pub struct NodeRenderPass {
    pub nodes: Vec<Node>,
    pub pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
}

#[derive(Debug)]
pub struct Node {
    pub size: cgmath::Vector2<f32>,
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub color: cgmath::Vector4<f32>,
}

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct NodeRaw {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

impl NodeRenderPass {
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
        nodes: Vec<Node>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Node Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("node.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Node"),
            bind_group_layouts: &[camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Node Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), NodeRaw::desc()],
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
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(Self::VERTICES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "Node Index Buffer".into(),
            contents: bytemuck::cast_slice(Self::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut instance_vec: Vec<MaybeUninit<NodeRaw>> = (0..DEFAULT_INSTANCE_BUFFER_CAP)
            .map(|_| MaybeUninit::zeroed())
            .collect();
        for (i, node) in nodes.iter().enumerate() {
            instance_vec[i] = MaybeUninit::new(node.to_instance());
        }

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Node Instance Buffer"),
            size: (std::mem::size_of::<NodeRaw>() * DEFAULT_INSTANCE_BUFFER_CAP) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &instance_buffer,
            0,
            bytemuck::cast_slice(&nodes.iter().map(Node::to_instance).collect::<Vec<_>>()),
        );

        Self {
            nodes,
            pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
        }
    }

    pub fn write(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.nodes.iter().map(Node::to_instance).collect::<Vec<_>>()),
        );
    }

    pub fn add_node(&mut self, node: Node, queue: &wgpu::Queue) {
        let raw = node.to_instance();
        let idx = self.nodes.len();
        self.nodes.push(node);
        queue.write_buffer(
            &self.instance_buffer,
            (idx * std::mem::size_of::<NodeRaw>()) as u64,
            bytemuck::cast_slice(&[raw]),
        )
    }

    pub fn update_node(&mut self, idx: u32, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            (idx as usize * std::mem::size_of::<NodeRaw>()) as u64,
            bytemuck::cast_slice(&[self.nodes.get(idx as usize).unwrap().to_instance()]),
        )
    }

    pub fn render<'a, 'b>(
        &'a self,
        camera_bind_group: &'a wgpu::BindGroup,
        render_pass: &mut wgpu::RenderPass<'a>,
    ) {
        if self.nodes.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..Self::INDICES.len() as u32, 0, 0..self.nodes.len() as u32);
    }
}

impl Node {
    pub fn new<S, P, C>(size: S, pos: P, rotation: cgmath::Quaternion<f32>, color: C) -> Self
    where
        S: Into<cgmath::Vector2<f32>>,
        P: Into<cgmath::Vector3<f32>>,
        C: Into<cgmath::Vector4<f32>>,
    {
        Self {
            size: size.into(),
            position: pos.into(),
            rotation,
            color: color.into(),
        }
    }

    pub fn intersects(&self, pos: &cgmath::Vector3<f32>) -> bool {
        pos.x <= self.position.x + (self.size.x * 1.0)
            && pos.x >= self.position.x - (self.size.x * 1.0)
            && pos.y <= self.position.y + (self.size.y * 1.0)
            && pos.y >= self.position.y - (self.size.y * 1.0)
        // && self.position.x == pos.z
    }

    pub fn to_instance(&self) -> NodeRaw {
        NodeRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from_nonuniform_scale(self.size.x, self.size.y, 1.0)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
            color: self.color.into(),
        }
    }
}

impl NodeRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        // model matrix
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        // color
        6 => Float32x4,
    ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<NodeRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn size() {
        println!("size: {}", std::mem::size_of::<super::NodeRaw>());
    }
}
