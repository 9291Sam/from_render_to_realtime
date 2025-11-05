use std::sync::Arc;

use pollster::FutureExt;
use wgpu::{
    Adapter, Backends, CommandEncoderDescriptor, Device, FeaturesWGPU, FeaturesWebGPU, Instance,
    InstanceDescriptor, Limits, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration,
    wgt::DeviceDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window},
};

use crate::render::RenderContext;

pub enum App {
    Uninitialized,
    Initialized {
        window: Arc<Window>,
        _instance: Instance,
        surface: Surface<'static>,
        adapter: Adapter,
        device: Device,
        queue: Queue,
        render_context: Box<RenderContext>,
        is_cursor_grabbed: bool,
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
            .contains(&RenderContext::SURFACE_TEXTURE_FORMAT)
    );

    if new_size.height > 0 && new_size.width > 0 {
        surface.configure(
            device,
            &SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: RenderContext::SURFACE_TEXTURE_FORMAT,
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

fn set_cursor_grabbed(window: &Window, is_grabbed: bool) {
    if is_grabbed {
        window.set_cursor_visible(false);
        if let Err(_e) = window.set_cursor_grab(CursorGrabMode::Confined)
            && let Err(err) = window.set_cursor_grab(CursorGrabMode::Locked)
        {
            eprintln!("Could not confine or lock cursor: {:?}", err);
        }
    } else {
        window.set_cursor_visible(true);
        if let Err(err) = window.set_cursor_grab(CursorGrabMode::None) {
            eprintln!("Could not un-grab cursor: {:?}", err);
        }
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

            let mut render_context = RenderContext::new(device.clone());

            render_context.on_resume();
            let initial_size = window.inner_size();
            resize_surface(&surface, &adapter, &device, initial_size);
            render_context.resize(initial_size);

            let is_cursor_grabbed = true;
            set_cursor_grabbed(&window, is_cursor_grabbed);

            *self = App::Initialized {
                window,
                _instance: instance,
                surface,
                adapter,
                device,
                queue,
                render_context: Box::new(render_context),
                is_cursor_grabbed,
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

    fn device_event(
        &mut self,
        _: &winit::event_loop::ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let App::Initialized {
            render_context,
            is_cursor_grabbed,
            ..
        } = self
            && let winit::event::DeviceEvent::MouseMotion { delta: (dx, dy) } = event
            && *is_cursor_grabbed
        {
            render_context.on_mouse_motion(dx, dy);
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
            render_context,
            is_cursor_grabbed,
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
                render_context.resize(new_size);
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                let new_size = window.inner_size();
                resize_surface(surface, adapter, device, new_size);
                render_context.resize(new_size);
                window.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                if !*is_cursor_grabbed {
                    *is_cursor_grabbed = true;
                    set_cursor_grabbed(window, *is_cursor_grabbed);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key_code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                if key_code == KeyCode::Escape && key_state.is_pressed() {
                    event_loop.exit();
                } else if key_code == KeyCode::Backslash && key_state.is_pressed() {
                    *is_cursor_grabbed = !*is_cursor_grabbed;
                    set_cursor_grabbed(window, *is_cursor_grabbed);
                } else if *is_cursor_grabbed {
                    render_context.on_key_event(key_code, key_state);
                }
            }
            WindowEvent::RedrawRequested => {
                render_context.update();

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

                let mut command_encoder =
                    device.create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Command Encoder"),
                    });

                render_context.render_frame(
                    &mut command_encoder,
                    &surface_texture.texture.create_view(&Default::default()),
                    queue,
                );

                queue.submit([command_encoder.finish()]);

                surface_texture.present();

                window.request_redraw();
            }
            _ => {}
        }
    }
}
