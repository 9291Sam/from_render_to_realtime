use wgpu::{
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PrimitiveState, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, VertexState, include_wgsl,
};

pub struct Triangle {
    triangle_pipeline: RenderPipeline,
}

impl Triangle {
    pub fn new(device: Device) -> Self {
        let triangle_shader_module = device.create_shader_module(include_wgsl!("triangle.wgsl"));

        let triangle_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
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
                    format: wgpu::TextureFormat::Bgra8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::all(),
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self { triangle_pipeline }
    }

    pub fn draw(&mut self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.triangle_pipeline);
        render_pass.draw(0..3, 0..1);
    }
}
