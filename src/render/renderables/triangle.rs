use bytemuck::bytes_of;

use wgpu::{
    BindGroupLayout, ColorTargetState, ColorWrites, DepthBiasState, DepthStencilState, Device,
    FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, PushConstantRange, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, StencilState, TextureView, VertexState, include_wgsl,
};

use crate::render::{
    linear_algebra::{Camera, Transform},
    render_context::{FrameDataManager, RenderContext},
};

pub struct Triangle {
    triangle_pipeline: RenderPipeline,
    transform: Transform,
}

impl Triangle {
    pub fn new(
        device: Device,
        global_bind_group_layout: &BindGroupLayout,
        transform: Transform,
    ) -> Self {
        let triangle_shader_module = device.create_shader_module(include_wgsl!("triangle.wgsl"));

        let triangle_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Simple Triangle Pipeline"),
            layout: Some(&device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("Pipeline Layout"),
                bind_group_layouts: &[global_bind_group_layout],
                push_constant_ranges: &[PushConstantRange {
                    stages: ShaderStages::VERTEX,
                    range: 0..4,
                }],
            })),
            vertex: VertexState {
                module: &triangle_shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: RenderContext::SURFACE_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &triangle_shader_module,
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

        Self {
            triangle_pipeline,
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
        render_pass.set_pipeline(&self.triangle_pipeline);
        render_pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            bytes_of(&frame_data_manager.append_transform(self.transform.clone())),
        );
        render_pass.draw(0..3, 0..1);
    }
}
