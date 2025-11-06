use std::{
    f32::consts::PI,
    fs::File,
    io::BufReader,
    ops::Rem,
    time::{Duration, Instant},
};

use bytemuck::{Pod, Zeroable, cast_slice};
use nalgebra_glm::{Mat4, Vec2, Vec3, Vec4};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBinding, BufferBindingType, BufferUsages,
    Color, CommandEncoder, Device, Extent3d, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderStages, StoreOp, Texture,
    TextureDescriptor, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
    wgt::BufferDescriptor,
};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

use crate::render::{
    linear_algebra::{Camera, Transform},
    renderables::{BillBoard, BillBoardManager, MeshDrawer, Triangle},
};

pub struct RenderContext {
    init_time: Instant,
    device: Device,

    camera: Camera,
    input_state: InputState,
    last_frame_time: Option<Instant>,
    depth_texture: Option<Texture>,
    depth_view: Option<TextureView>,

    mvp_matrices: Buffer,
    model_matrices: Buffer,
    normal_matrices: Buffer,
    point_lights: Buffer,
    global_data_bind_group: BindGroup,

    frame_data_manager: FrameDataManager,

    triangles: Vec<Triangle>,
    meshes: Vec<MeshDrawer>,
    billboard_manager: BillBoardManager,
}

impl RenderContext {
    pub const SURFACE_TEXTURE_FORMAT: TextureFormat = TextureFormat::Bgra8UnormSrgb;
    pub const SURFACE_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;
    pub const MAX_MATRICES: u64 = 1024;
    pub const MAX_POINT_LIGHTS: u64 = 64;

    pub fn new(device: Device) -> RenderContext {
        let mvp_matrices = device.create_buffer(&BufferDescriptor {
            label: Some("Global Projection Matrices Buffer"),
            size: size_of::<Mat4>() as u64 * Self::MAX_MATRICES,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let model_matrices = device.create_buffer(&BufferDescriptor {
            label: Some("Global Model Matrices Buffer"),
            size: size_of::<Mat4>() as u64 * Self::MAX_MATRICES,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let normal_matrices = device.create_buffer(&BufferDescriptor {
            label: Some("Global Norma Matrices Buffer"),
            size: size_of::<Mat4>() as u64 * Self::MAX_MATRICES,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let point_lights = device.create_buffer(&BufferDescriptor {
            label: Some("Global Point Lights Buffer"),
            size: size_of::<PointLight>() as u64 * Self::MAX_POINT_LIGHTS,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let global_data_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Bind Group Layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::all(),
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let global_data_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &global_data_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &mvp_matrices,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &model_matrices,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &normal_matrices,
                        offset: 0,
                        size: None,
                    }),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(BufferBinding {
                        buffer: &point_lights,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });

        RenderContext {
            init_time: Instant::now(),
            device: device.clone(),

            camera: Camera::new(Vec3::new(-4.030, 7.04, -10.5692), 0.507993, 0.339997),
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
                        translation: Vec3::new(-8.3, 3.8723, 2.1),
                        ..Default::default()
                    },
                ),
                Triangle::new(
                    device.clone(),
                    &global_data_bind_group_layout,
                    Transform {
                        translation: Vec3::new(-7.20, 4.3, -1.1),
                        ..Default::default()
                    },
                ),
            ],
            meshes: vec![
                MeshDrawer::new(
                    device.clone(),
                    &global_data_bind_group_layout,
                    BufReader::new(File::open("models/quad.obj").unwrap()),
                    Transform {
                        scale: Vec3::new(100.0, -100.0, 100.0),
                        ..Default::default()
                    },
                ),
                MeshDrawer::new(
                    device.clone(),
                    &global_data_bind_group_layout,
                    BufReader::new(File::open("models/flat_vase.obj").unwrap()),
                    Transform {
                        translation: Vec3::new(-3.25, 0.0, 0.0),
                        scale: Vec3::new(10.0, -10.0, 10.0),
                        ..Default::default()
                    },
                ),
                MeshDrawer::new(
                    device.clone(),
                    &global_data_bind_group_layout,
                    BufReader::new(File::open("models/smooth_vase.obj").unwrap()),
                    Transform {
                        translation: Vec3::new(3.25, 0.0, 0.0),
                        scale: Vec3::new(10.0, -10.0, 10.0),
                        ..Default::default()
                    },
                ),
                MeshDrawer::new(
                    device.clone(),
                    &global_data_bind_group_layout,
                    BufReader::new(File::open("models/beetle.obj").unwrap()),
                    Transform {
                        translation: Vec3::new(0.0, -3.0, 0.0),
                        scale: Vec3::new(10.0, 10.0, 10.0),
                        ..Default::default()
                    },
                ),
                // MeshDrawer::new(
                //     device.clone(),
                //     &global_data_bind_group_layout,
                //     BufReader::new(File::open("models/suzanne.obj").unwrap()),
                //     Transform {
                //         translation: Vec3::new(0.0, 0.0, 0.0),
                //         scale: Vec3::new(10.0, 10.0, 10.0),
                //         ..Default::default()
                //     },
                // ),
            ],
            billboard_manager: BillBoardManager::new(device, &global_data_bind_group_layout),
            mvp_matrices,
            model_matrices,
            normal_matrices,
            point_lights,
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

        let camera_move_speed: f32 = if self.input_state.sprint { 12.0 } else { 5.0 };
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

        let time_alive: f32 = self.init_time.elapsed().as_secs_f32();

        fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
            if s == 0.0 {
                return Vec3::new(v, v, v);
            }

            let h_norm = (h * 6.0).rem(6.0);
            let i = h_norm.floor() as i32;
            let f = h_norm - h_norm.floor();

            let p = v * (1.0 - s);
            let q = v * (1.0 - f * s);
            let t = v * (1.0 - (1.0 - f) * s);

            match i {
                0 => Vec3::new(v, t, p),
                1 => Vec3::new(q, v, p),
                2 => Vec3::new(p, v, t),
                3 => Vec3::new(p, q, v),
                4 => Vec3::new(t, p, v),
                _ => Vec3::new(v, p, q),
            }
        }

        const NUMBER_OF_POINT_LIGHTS: u32 = 64;
        let point_lights: Vec<PointLight> = (0..NUMBER_OF_POINT_LIGHTS)
            .map(|i| {
                let t = time_alive + (i as f32 / NUMBER_OF_POINT_LIGHTS as f32) * PI * 2.0;
                let c = hsv_to_rgb(i as f32 / NUMBER_OF_POINT_LIGHTS as f32, 1.0, 1.0);
                let r = 16.0;

                PointLight {
                    position: Vec4::new(
                        r * t.cos(),
                        (t * 3.0 + 3.0 * time_alive).cos() * 2.83 + 5.21,
                        r * t.sin(),
                        0.0,
                    ),
                    color_and_intensity: Vec4::new(c.x, c.y, c.z, 3.25),
                }
            })
            .collect::<Vec<_>>();

        self.billboard_manager.set_billboards(
            point_lights
                .iter()
                .map(|p| BillBoard {
                    position_and_size: Vec4::new(p.position.x, p.position.y, p.position.z, 0.5),
                    color: Vec4::new(
                        p.color_and_intensity.x,
                        p.color_and_intensity.y,
                        p.color_and_intensity.z,
                        0.0,
                    ),
                })
                .collect(),
        );

        self.billboard_manager.draw(
            &self.camera,
            &mut self.frame_data_manager,
            queue,
            &mut render_pass,
        );

        let surface_extent = surface_view.texture().size();

        queue.write_buffer(
            &self.mvp_matrices,
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

        queue.write_buffer(
            &self.model_matrices,
            0,
            cast_slice(
                &self
                    .frame_data_manager
                    .end_frame()
                    .iter()
                    .map(|t| t.as_model_matrix())
                    .collect::<Vec<Mat4>>(),
            ),
        );

        queue.write_buffer(
            &self.normal_matrices,
            0,
            cast_slice(
                &self
                    .frame_data_manager
                    .end_frame()
                    .iter()
                    .map(|t| t.as_normal_matrix())
                    .collect::<Vec<Mat4>>(),
            ),
        );

        queue.write_buffer(&self.point_lights, 0, cast_slice(&point_lights));
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

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default, Debug)]
struct PointLight {
    position: Vec4,
    color_and_intensity: Vec4,
}
