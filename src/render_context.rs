use wgpu::{
    Color, ColorTargetState, ColorWrites, CommandEncoder, Device, FragmentState, MultisampleState,
    Operations, PipelineCompilationOptions, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, TextureView, VertexState,
    include_wgsl,
};
use winit::dpi::PhysicalSize;

use crate::app::App;

pub struct RenderContext {
    _device: Device,
    triangle_pipeline: RenderPipeline,
}

impl RenderContext {
    pub fn new(device: Device) -> RenderContext {
        let triangle_shaders = include_wgsl!("triangle.wgsl");
        let triangle_shader_module = device.create_shader_module(triangle_shaders);

        RenderContext {
            _device: device.clone(),
            triangle_pipeline: device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Simple Triangle Pipeline"),
                layout: None,
                vertex: VertexState {
                    module: &triangle_shader_module,
                    entry_point: Some("vs_main"),
                    compilation_options: PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                fragment: Some(FragmentState {
                    module: &triangle_shader_module,
                    entry_point: Some("fs_main"),
                    compilation_options: PipelineCompilationOptions::default(),
                    targets: &[Some(ColorTargetState {
                        format: App::SURFACE_TEXTURE_FORMAT,
                        blend: None,
                        write_mask: ColorWrites::all(),
                    })],
                }),
                multiview: None,
                cache: None,
            }),
        }
    }

    pub fn resize(&mut self, _new_size: PhysicalSize<u32>) {}

    pub fn render_frame(
        &mut self,
        command_encoder: &mut CommandEncoder,
        surface_view: &TextureView,
    ) {
        let mut render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: surface_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: wgpu::LoadOp::Clear(Color {
                        r: 0.5,
                        g: 0.6,
                        b: 0.7,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.triangle_pipeline);
        render_pass.draw(0..3, 0..1);
    }
}
