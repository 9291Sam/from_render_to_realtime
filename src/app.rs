use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

pub struct App {
    window: Option<Window>,
}

impl App {
    pub fn new() -> App {
        App { window: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
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
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {
                println!("{event_loop:?} {window_id:?} {event:?}")
            }
        }
    }
}
