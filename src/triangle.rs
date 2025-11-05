use std::f32::consts::PI;

use bytemuck::bytes_of;
use nalgebra_glm::{Mat4, Vec2};

use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferUsages,
    ColorTargetState, ColorWrites, DepthBiasState, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue,
    RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderStages, StencilState, TextureView,
    VertexState, include_wgsl, wgt::BufferDescriptor,
};

use crate::{
    camera::{Camera, Transform},
    render_context::RenderContext,
};

pub struct Triangle {
    triangle_pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    transform: Transform,
}

impl Triangle {
    pub fn new(device: Device, transform: Transform) -> Self {
        let triangle_shaders = include_wgsl!("triangle.wgsl");
        let triangle_shader_module = device.create_shader_module(triangle_shaders);

        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Triangle Camera Uniform Buffer"),
            size: size_of::<Mat4>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        let triangle_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Simple Triangle Pipeline"),
            layout: Some(&pipeline_layout),
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
            uniform_buffer,
            bind_group,
            transform,
        }
    }

    pub fn draw(
        &mut self,
        camera: &Camera,
        surface_view: &TextureView,
        queue: &Queue,
        render_pass: &mut RenderPass,
    ) {
        let surface_extent = surface_view.texture().size();

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytes_of(&camera.get_perspective(
                Vec2::new(surface_extent.width as f32, surface_extent.height as f32),
                70.0 * PI / 180.0,
                &self.transform,
            )),
        );

        render_pass.set_pipeline(&self.triangle_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
