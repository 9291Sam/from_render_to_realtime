use std::{
    f32::consts::PI,
    time::{Duration, Instant},
};

use bytemuck::bytes_of;
use nalgebra_glm::{Mat4, Vec2, Vec3};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferBinding, BufferUsages, Color, ColorTargetState,
    ColorWrites, CommandEncoder, Device, FragmentState, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, TextureView, VertexState, include_wgsl, wgt::BufferDescriptor,
};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

use crate::{
    app::App,
    camera::{Camera, Transform},
};

pub struct RenderContext {
    _device: Device,
    triangle_pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    bind_group: BindGroup,

    camera: Camera,
    input_state: InputState,
    last_frame_time: Option<Instant>,
}

impl RenderContext {
    pub fn new(device: Device) -> RenderContext {
        let triangle_shaders = include_wgsl!("triangle.wgsl");
        let triangle_shader_module = device.create_shader_module(triangle_shaders);

        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Camera Uniform Buffer"),
            size: size_of::<Mat4>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
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

        RenderContext {
            _device: device.clone(),
            triangle_pipeline: device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("Simple Triangle Pipeline"),
                layout: Some(&pipeline_layout),
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
            camera: Camera::new_zeroed(),
            input_state: InputState::default(),
            last_frame_time: None,
            uniform_buffer,
            bind_group,
        }
    }

    pub fn on_resume(&mut self) {
        self.last_frame_time = Some(Instant::now());
    }

    pub fn on_key_event(&mut self, key_code: KeyCode, state: ElementState) {
        let is_pressed = state.is_pressed();
        match key_code {
            KeyCode::KeyW | KeyCode::ArrowUp => self.input_state.forward = is_pressed,
            KeyCode::KeyS | KeyCode::ArrowDown => self.input_state.backward = is_pressed,
            KeyCode::KeyA | KeyCode::ArrowLeft => self.input_state.left = is_pressed,
            KeyCode::KeyD | KeyCode::ArrowRight => self.input_state.right = is_pressed,
            KeyCode::Space => self.input_state.up = is_pressed,
            KeyCode::ControlLeft => self.input_state.down = is_pressed,
            KeyCode::ShiftRight | KeyCode::ShiftLeft => self.input_state.sprint = is_pressed,
            KeyCode::KeyK => self.input_state.log_camera = is_pressed,
            _ => {}
        }
    }

    pub fn on_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        const MOUSE_SENSITIVITY: f32 = 0.004;

        self.camera.add_yaw((delta_x as f32) * MOUSE_SENSITIVITY);
        self.camera.add_pitch((delta_y as f32) * MOUSE_SENSITIVITY);
    }

    pub fn resize(&mut self, _new_size: PhysicalSize<u32>) {}

    pub fn update(&mut self) {
        let delta_time = if let Some(last_frame_time) = self.last_frame_time {
            Instant::now() - last_frame_time
        } else {
            Duration::from_secs(0)
        };
        self.last_frame_time = Some(Instant::now());
        let delta_time_secs = delta_time.as_secs_f32();

        let camera_move_speed: f32 = if self.input_state.sprint { 5.0 } else { 2.0 };
        let mut delta_pos = Vec3::zeros();

        if self.input_state.forward {
            delta_pos += self.camera.get_forward_vector();
        }
        if self.input_state.backward {
            delta_pos -= self.camera.get_forward_vector();
        }
        if self.input_state.right {
            delta_pos += self.camera.get_right_vector();
        }
        if self.input_state.left {
            delta_pos -= self.camera.get_right_vector();
        }

        if self.input_state.up {
            delta_pos += *Transform::global_up_vector();
        }
        if self.input_state.down {
            delta_pos -= *Transform::global_up_vector();
        }

        if delta_pos.norm_squared() > 0.0 {
            delta_pos = delta_pos.normalize() * camera_move_speed * delta_time_secs;
        }

        self.camera.add_position(delta_pos);

        if self.input_state.log_camera {
            println!(
                "Camera: {} | FPS: {:.1}",
                self.camera,
                1.0 / (delta_time_secs)
            );
        }
    }

    pub fn render_frame(
        &mut self,
        command_encoder: &mut CommandEncoder,
        surface_view: &TextureView,
        queue: &mut Queue,
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

        let surface_extent = surface_view.texture().size();

        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytes_of(&self.camera.get_perspective(
                Vec2::new(surface_extent.width as f32, surface_extent.height as f32),
                70.0 * PI / 180.0,
                &Transform::new(),
            )),
        );

        render_pass.set_pipeline(&self.triangle_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

#[derive(Default)]
pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    pub log_camera: bool,
}
