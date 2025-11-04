use std::{
    f32::consts::PI,
    fs::File,
    io::BufReader,
    time::{Duration, Instant},
};

use bytemuck::bytes_of;
use nalgebra_glm::{Mat4, Vec2, Vec3};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, Buffer, BufferBinding, BufferUsages, Color, ColorTargetState,
    ColorWrites, CommandEncoder, Device, FragmentState, LoadOp, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, StoreOp, TextureView, VertexState, include_wgsl, wgt::BufferDescriptor,
};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

use crate::{
    app::App,
    camera::{Camera, Transform},
    mesh_drawer::MeshDrawer,
    triangle::Triangle,
};

pub struct RenderContext {
    _device: Device,

    camera: Camera,
    input_state: InputState,
    last_frame_time: Option<Instant>,

    triangle: Triangle,
    triangle2: Triangle,
    mesh: MeshDrawer,
}

impl RenderContext {
    pub fn new(device: Device) -> RenderContext {
        RenderContext {
            _device: device.clone(),

            camera: Camera::new_zeroed(),
            input_state: InputState::default(),
            last_frame_time: None,
            triangle: Triangle::new(
                device.clone(),
                Transform {
                    translation: Vec3::new(3.0, -1.3, 2.1),
                    ..Default::default()
                },
            ),

            triangle2: Triangle::new(
                device.clone(),
                Transform {
                    translation: Vec3::new(3.0, 1.3, 2.1),
                    ..Default::default()
                },
            ),
            mesh: MeshDrawer::new(
                device,
                BufReader::new(File::open("models/quad.obj").unwrap()),
                Transform {
                    scale: Vec3::new(2.0, -2.0, 2.0),
                    ..Default::default()
                },
            ),
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
                    load: LoadOp::Clear(Color {
                        r: 0.5,
                        g: 0.6,
                        b: 0.7,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        self.triangle
            .draw(&self.camera, surface_view, queue, &mut render_pass);

        self.triangle2
            .draw(&self.camera, surface_view, queue, &mut render_pass);

        self.mesh
            .draw(&self.camera, surface_view, queue, &mut render_pass);
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
