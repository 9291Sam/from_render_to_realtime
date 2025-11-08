use std::sync::Arc;

use pollster::FutureExt;
use wgpu::{
    Adapter, Backends, Color, CommandEncoderDescriptor, Device, FeaturesWGPU, FeaturesWebGPU,
    Instance, InstanceDescriptor, Limits, LoadOp, Operations, Queue, RenderPassColorAttachment,
    RenderPassDescriptor, RequestAdapterOptions, StoreOp, Surface, SurfaceConfiguration,
    TextureFormat,
    wgt::{DeviceDescriptor, TextureViewDescriptor},
};
use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, event::WindowEvent, window::Window,
};

use crate::triangle::Triangle;

pub enum App {
    Uninitialized,
    Initialized {
        window: Arc<Window>,
        _instance: Instance,
        surface: Surface<'static>,
        adapter: Adapter,
        device: Device,
        queue: Queue,
        triangle: Triangle,
    },
}

impl App {
    pub fn new() -> App {
        App::Uninitialized
    }
}

fn resize_surface(
    surface: &Surface,
    adapter: &Adapter,
    device: &Device,
    new_size: PhysicalSize<u32>,
) {
    let surface_capabilities = surface.get_capabilities(adapter);
    assert!(
        surface_capabilities
            .formats
            .contains(&TextureFormat::Bgra8UnormSrgb)
    );

    if new_size.height > 0 && new_size.width > 0 {
        surface.configure(
            device,
            &SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: TextureFormat::Bgra8UnormSrgb,
                width: new_size.width,
                height: new_size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: surface_capabilities.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let App::Uninitialized = self {
            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes().with_title("From Render To Real Time"),
                    )
                    .expect("Failed to create window!"),
            );

            let instance = Instance::new(&InstanceDescriptor {
                backends: Backends::PRIMARY,
                ..Default::default()
            });

            let surface: Surface<'static> = instance
                .create_surface(window.clone())
                .expect("Unable to create surface from window!");

            let adapter = instance
                .request_adapter(&RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .block_on()
                .expect("Unable to request adapter!");

            let (device, queue) = adapter
                .request_device(&DeviceDescriptor {
                    label: Some("From Render to Real Time"),
                    required_features: wgpu::Features {
                        features_wgpu: FeaturesWGPU::PUSH_CONSTANTS,
                        features_webgpu: FeaturesWebGPU::default(),
                    },
                    required_limits: Limits {
                        max_texture_dimension_1d: 8192,
                        max_texture_dimension_2d: 8192,
                        max_texture_dimension_3d: 2048,
                        max_push_constant_size: 128,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .block_on()
                .expect("Failed to create WGPU Device & Queue");

            let initial_size = window.inner_size();
            resize_surface(&surface, &adapter, &device, initial_size);

            *self = App::Initialized {
                window,
                _instance: instance,
                surface,
                adapter,
                queue,
                triangle: Triangle::new(device.clone()),
                device,
            };
        } else if let App::Initialized {
            surface,
            adapter,
            device,
            window,
            ..
        } = self
        {
            resize_surface(surface, adapter, device, window.inner_size());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let App::Initialized {
            window,
            surface,
            adapter,
            device,
            queue,
            triangle,
            ..
        } = self
        else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                println!("Close Requested!");
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                resize_surface(surface, adapter, device, new_size);
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let new_size = window.inner_size();
                resize_surface(surface, adapter, device, new_size);
                window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let surface_texture = match surface.get_current_texture() {
                    Ok(t) => t,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        window.request_redraw();
                        return;
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        panic!("Surface Error: OutOfMemory");
                    }
                    Err(e) => {
                        eprintln!("Unhandled Surface Error: {:?}", e);
                        window.request_redraw();
                        return;
                    }
                };

                let surface_view = surface_texture
                    .texture
                    .create_view(&TextureViewDescriptor::default());

                let mut command_encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Command Encoder"),
                    });

                let mut render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &surface_view,
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

                triangle.draw(&mut render_pass);

                drop(render_pass);

                queue.submit([command_encoder.finish()]);

                surface_texture.present();

                window.request_redraw();
            }
            _ => {}
        }
    }
}
