use std::io::BufRead;

use bytemuck::{Pod, Zeroable, bytes_of, cast_slice};
use nalgebra_glm::Vec3;
use obj::Obj;
use wgpu::{
    BindGroupLayout, Buffer, BufferAddress, BufferUsages, ColorTargetState, ColorWrites,
    DepthBiasState, DepthStencilState, Device, FragmentState, FrontFace, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology,
    PushConstantRange, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages,
    StencilState, TextureView, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
    VertexStepMode, include_wgsl,
    util::{BufferInitDescriptor, DeviceExt},
};

use crate::render::{
    linear_algebra::{Camera, Transform},
    render_context::{FrameDataManager, RenderContext},
};

pub struct MeshDrawer {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    number_of_indices: u32,
    pipeline: RenderPipeline,
    transform: Transform,
}

impl MeshDrawer {
    pub fn new(
        device: Device,
        global_bind_group_layout: &BindGroupLayout,
        input: impl BufRead,
        transform: Transform,
    ) -> MeshDrawer {
        let o: Obj = obj::load_obj(input).unwrap();

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Mesh Drawer Vertex Buffer"),
            contents: cast_slice(
                &o.vertices
                    .iter()
                    .map(|v| MeshVertex {
                        position: Vec3::new(v.position[0], v.position[1], v.position[2]),
                        normal: Vec3::new(v.normal[0], v.normal[1], v.normal[2]),
                    })
                    .collect::<Vec<_>>(),
            ),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Mesh Drawer Index Buffer"),
            contents: cast_slice(&o.indices),
            usage: BufferUsages::INDEX,
        });

        let shader_module = device.create_shader_module(include_wgsl!("mesh.wgsl"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Mesh Pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[global_bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..4,
                }],
            })),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[MeshVertex::desc()],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: RenderContext::SURFACE_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: RenderContext::SURFACE_TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            multiview: None,
            cache: None,
        });

        MeshDrawer {
            vertex_buffer,
            index_buffer,
            number_of_indices: o.indices.len().try_into().unwrap(),
            pipeline,
            transform,
        }
    }

    pub fn draw(
        &mut self,
        _: &Camera,
        frame_data_manager: &mut FrameDataManager,
        _: &TextureView,
        _: &Queue,
        render_pass: &mut RenderPass,
    ) {
        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        render_pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            bytes_of(&frame_data_manager.append_transform(self.transform.clone())),
        );

        render_pass.draw_indexed(0..self.number_of_indices, 0, 0..1);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct MeshVertex {
    position: Vec3,
    normal: Vec3,
}

impl MeshVertex {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<MeshVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: VertexFormat::Float32x3,
                },
                VertexAttribute {
                    offset: size_of::<Vec3>() as BufferAddress,
                    shader_location: 1,
                    format: VertexFormat::Float32x3,
                },
            ],
        }
    }
}
