use std::sync::Arc;

use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

pub enum App {
    Uninitialized,
    Initialized { window: Arc<Window> },
}

impl App {
    pub fn new() -> App {
        App::Uninitialized
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

            *self = App::Initialized { window };
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let App::Initialized { window } = self else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                println!("Close Requested!");
                event_loop.exit();
            }
            WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. }
            | WindowEvent::RedrawRequested => {
                window.request_redraw();
            }
            _ => {}
        }
    }
}
