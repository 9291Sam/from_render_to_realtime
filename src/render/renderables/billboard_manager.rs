use bytemuck::{Pod, Zeroable, bytes_of, cast_slice};
use nalgebra_glm::{Vec3, Vec4};
use rand::RngCore;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages,
    ColorTargetState, ColorWrites, DepthBiasState, DepthStencilState, Device, FragmentState,
    FrontFace, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PrimitiveTopology, PushConstantRange, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderStages, StencilState, VertexState, include_wgsl,
};

use crate::render::{
    linear_algebra::{Camera, Transform},
    render_context::{FrameDataManager, RenderContext},
};

pub struct BillBoardManager {
    pipeline: RenderPipeline,
    billboard_buffer: Buffer,
    billboard_bind_group: BindGroup,
    billboard_update: Option<Vec<BillBoard>>,
    number_of_billboards_to_draw: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod, PartialEq, PartialOrd)]
pub struct BillBoard {
    pub position_and_size: Vec4,
    pub color: Vec4,
}

impl BillBoardManager {
    pub const MAX_BILLBOARDS: u64 = 1 << 16;

    pub fn new(device: Device, global_bind_group_layout: &BindGroupLayout) -> BillBoardManager {
        let billboard_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Billboard Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let billboard_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Billboard Storage Buffer"),
            size: Self::MAX_BILLBOARDS * size_of::<BillBoard>() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let billboard_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Billboard Bind Group"),
            layout: &billboard_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: billboard_buffer.as_entire_binding(),
            }],
        });

        let shader_module = device.create_shader_module(include_wgsl!("billboard.wgsl"));

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Billboard Pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Billboard Pipeline Layout"),
                bind_group_layouts: &[global_bind_group_layout, &billboard_bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX_FRAGMENT,
                    range: 0..40,
                }],
            })),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
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

        BillBoardManager {
            pipeline,
            billboard_buffer,
            billboard_bind_group,
            billboard_update: None,
            number_of_billboards_to_draw: 0,
        }
    }

    pub fn set_billboards(&mut self, new_billboards: Vec<BillBoard>) {
        self.billboard_update = Some(new_billboards);
    }

    pub fn draw(
        &mut self,
        camera: &Camera,
        frame_data_manager: &mut FrameDataManager,
        q: &Queue,
        render_pass: &mut RenderPass,
    ) {
        #[repr(C)]
        #[derive(Clone, Copy, Pod, Zeroable)]
        struct PushConstantData {
            camera_right: Vec3,
            pad: f32,
            camera_up: Vec3,
            pad2: f32,
            random_seed: u32,
            matrix_index: u32,
        }

        if let Some(new_billboards) = self.billboard_update.take() {
            q.write_buffer(&self.billboard_buffer, 0, cast_slice(&new_billboards));
            self.number_of_billboards_to_draw = new_billboards.len().try_into().unwrap();
        }

        if self.number_of_billboards_to_draw > 0 {
            render_pass.set_pipeline(&self.pipeline);

            render_pass.set_bind_group(1, Some(&self.billboard_bind_group), &[]);

            render_pass.set_push_constants(
                ShaderStages::VERTEX_FRAGMENT,
                0,
                bytes_of(&PushConstantData {
                    camera_right: camera.get_right_vector(),
                    camera_up: camera.get_up_vector(),
                    random_seed: rand::rng().next_u32(),
                    matrix_index: frame_data_manager.append_transform(Transform::new()),
                    pad: 0.0,
                    pad2: 0.0,
                }),
            );

            render_pass.draw(0..(self.number_of_billboards_to_draw * 6), 0..1);
        }
    }
}
