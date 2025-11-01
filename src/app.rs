use std::sync::Arc;

use pollster::FutureExt;
use wgpu::{
    Adapter, Backends, Color, CommandEncoderDescriptor, Device, ExperimentalFeatures, FeaturesWGPU,
    FeaturesWebGPU, Instance, InstanceDescriptor, Limits, Operations, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface,
    SurfaceConfiguration, wgt::DeviceDescriptor,
};
use winit::{
    application::ApplicationHandler, dpi::PhysicalSize, event::WindowEvent, window::Window,
};

pub struct App {
    window: Option<Arc<Window>>,
    wgpu_structures: Option<(Instance, Surface<'static>, Adapter, Device, Queue)>,
}

impl App {
    pub fn new() -> App {
        App {
            window: None,
            wgpu_structures: None,
        }
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.height > 0 && new_size.width > 0 {
            let (_, surface, adapter, device, _) = self
                .wgpu_structures
                .as_ref()
                .expect("BUG: tried to call resize w/o wgpu initialization");

            let surface_capabilities = surface.get_capabilities(adapter);
            let config = SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_capabilities.formats[0],
                width: new_size.width,
                height: new_size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: surface_capabilities.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            surface.configure(device, &config);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let is_first_init = self.window.is_none();

        if is_first_init {
            let window = Arc::new(
                event_loop
                    .create_window(Window::default_attributes())
                    .expect("Failed to create window!"),
            );

            self.window = Some(window.clone());

            let instance = Instance::new(&InstanceDescriptor {
                backends: Backends::PRIMARY,
                ..Default::default()
            });

            let surface: Surface<'static> = instance
                .create_surface(window.clone())
                .expect("Unable to create surface from window!");

            let adapter = instance
                .request_adapter(&RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: Some(&surface),
                })
                .block_on()
                .expect("Unable to request adapter!");

            let (device, queue) = adapter
                .request_device(&DeviceDescriptor {
                    label: Some("From Render to Real Time"),
                    required_features: wgpu::Features {
                        features_wgpu: FeaturesWGPU::empty(),
                        features_webgpu: FeaturesWebGPU::empty(),
                    },
                    required_limits: Limits::downlevel_defaults(),
                    experimental_features: ExperimentalFeatures::disabled(),
                    memory_hints: wgpu::MemoryHints::Performance,
                    trace: wgpu::Trace::Off,
                })
                .block_on()
                .expect("Failed to create WGPU Device & Queue");

            self.wgpu_structures = Some((instance, surface, adapter, device, queue))
        }

        self.resize(self.window.as_ref().unwrap().inner_size());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Close Requested!");
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                self.resize(new_size);
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.resize(self.window.as_ref().unwrap().inner_size());
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                let (_instance, surface, _adapter, device, queue) =
                    self.wgpu_structures.as_ref().unwrap();

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
                let surface_view = surface_texture.texture.create_view(&Default::default());

                let mut command_encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Command Encoder"),
                    });

                {
                    let _render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: &surface_view,
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
                }

                queue.submit([command_encoder.finish()]);

                surface_texture.present();

                window.request_redraw();
            }
            _ => {
                println!("{event_loop:?} {window_id:?} {event:?}")
            }
        }
    }
}
