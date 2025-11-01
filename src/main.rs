mod app;

use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().expect("Unable to create EventLoop!");

    let mut app = app::App::new();

    if let Err(error) = event_loop.run_app(&mut app) {
        panic!("Event Loop Error! {}", error);
    }
}
