use std::{
    f32::consts::PI,
    fs::File,
    io::BufReader,
    time::{Duration, Instant},
};

use bytemuck::cast_slice;
use nalgebra_glm::{Mat4, Vec2, Vec3};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferUsages,
    Color, CommandEncoder, Device, Extent3d, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages, StoreOp, Texture,
    TextureDescriptor, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    wgt::BufferDescriptor,
};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

use crate::render::{
    linear_algebra::{Camera, Transform},
    renderables::{MeshDrawer, Triangle},
};

pub struct RenderContext {
    device: Device,

    camera: Camera,
    input_state: InputState,
    last_frame_time: Option<Instant>,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,

    projection_matrices: Buffer,
    global_data_bind_group_layout: BindGroupLayout,
    global_data_bind_group: BindGroup,

    frame_data_manager: FrameDataManager,

    triangles: Vec<Triangle>,
    meshes: Vec<MeshDrawer>,
}

impl RenderContext {
    pub const SURFACE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const SURFACE_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
    pub const MAX_PROJECTION_MATRICES: u64 = 1024;

    pub fn new(device: Device) -> RenderContext {
        let projection_matrices = device.create_buffer(&BufferDescriptor {
            label: Some("Global Projection Matrices Buffer"),
            size: size_of::<Mat4>() as u64 * Self::MAX_PROJECTION_MATRICES,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let global_data_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::all(),
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let global_data_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &global_data_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(BufferBinding {
                    buffer: &projection_matrices,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        RenderContext {
            device: device.clone(),

            camera: Camera::new_zeroed(),
            input_state: InputState::default(),
            last_frame_time: None,
            depth_texture: None,
            depth_view: None,

            frame_data_manager: FrameDataManager::new(),

            triangles: vec![
                Triangle::new(
                    device.clone(),
                    &global_data_bind_group_layout,
                    Transform {
                        translation: Vec3::new(3.0, -1.3, 2.1),
                        ..Default::default()
                    },
                ),
                Triangle::new(
                    device.clone(),
                    &global_data_bind_group_layout,
                    Transform {
                        translation: Vec3::new(3.0, 1.3, 2.1),
                        ..Default::default()
                    },
                ),
            ],
            meshes: vec![MeshDrawer::new(
                device,
                &global_data_bind_group_layout,
                BufReader::new(File::open("models/quad.obj").unwrap()),
                Transform {
                    scale: Vec3::new(20.0, -20.0, 20.0),
                    ..Default::default()
                },
            )],

            projection_matrices,
            global_data_bind_group_layout,
            global_data_bind_group,
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
                sample_count: 1,
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
        let now = Instant::now();
        let delta_time = if let Some(last_frame_time) = self.last_frame_time {
            now - last_frame_time
        } else {
            Duration::from_secs(0)
        };
        self.last_frame_time = Some(now);
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

        render_pass.set_bind_group(0, Some(&self.global_data_bind_group), &[]);

        self.frame_data_manager.start_new_frame();

        for t in &mut self.triangles {
            t.draw(
                &self.camera,
                &mut self.frame_data_manager,
                surface_view,
                queue,
                &mut render_pass,
            );
        }

        for m in &mut self.meshes {
            m.draw(
                &self.camera,
                &mut self.frame_data_manager,
                surface_view,
                queue,
                &mut render_pass,
            );
        }

        let surface_extent = surface_view.texture().size();

        queue.write_buffer(
            &self.projection_matrices,
            0,
            cast_slice(
                &self
                    .frame_data_manager
                    .end_frame()
                    .iter()
                    .map(|t| {
                        self.camera.get_perspective(
                            Vec2::new(surface_extent.width as f32, surface_extent.height as f32),
                            70.0 * PI / 180.0,
                            t,
                        )
                    })
                    .collect::<Vec<Mat4>>(),
            ),
        );
    }
}

pub struct FrameDataManager {
    transforms: Vec<Transform>,
}

impl FrameDataManager {
    fn new() -> FrameDataManager {
        FrameDataManager { transforms: vec![] }
    }

    fn start_new_frame(&mut self) {
        self.transforms.clear();
    }

    #[must_use]
    pub fn append_transform(&mut self, transform: Transform) -> u32 {
        let output_index = self.transforms.len().try_into().unwrap();

        self.transforms.push(transform);

        output_index
    }

    fn end_frame(&self) -> &'_ [Transform] {
        &self.transforms
    }
}

#[derive(Default)]
struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    pub log_camera: bool,
}
