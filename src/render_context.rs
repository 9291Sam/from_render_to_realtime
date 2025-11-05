use std::{
    fs::File,
    io::BufReader,
    time::{Duration, Instant},
};

use nalgebra_glm::Vec3;
use wgpu::{
    Color, CommandEncoder, Device, Extent3d, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, Texture, TextureDescriptor,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

use crate::{
    camera::{Camera, Transform},
    mesh_drawer::MeshDrawer,
    triangle::Triangle,
};

pub struct RenderContext {
    device: Device,

    camera: Camera,
    input_state: InputState,
    last_frame_time: Option<Instant>,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,

    triangle: Triangle,
    triangle2: Triangle,
    mesh: MeshDrawer,
}

impl RenderContext {
    pub const SURFACE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const SURFACE_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn new(device: Device) -> RenderContext {
        RenderContext {
            device: device.clone(),

            camera: Camera::new_zeroed(),
            input_state: InputState::default(),
            last_frame_time: None,
            depth_texture: None,
            depth_view: None,

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

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            let depth_texture_desc = TextureDescriptor {
                label: Some("Depth Texture"),
                size: Extent3d {
                    width: new_size.width,
                    height: new_size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1, // Must match pipeline
                dimension: wgpu::TextureDimension::D2,
                format: Self::SURFACE_DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            };
            let depth_texture = self.device.create_texture(&depth_texture_desc);
            let depth_view = depth_texture.create_view(&TextureViewDescriptor::default());

            self.depth_texture = Some(depth_texture);
            self.depth_view = Some(depth_view);
        }
    }

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
        let depth_view = if let Some(view) = &self.depth_view {
            view
        } else {
            return;
        };

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
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
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
