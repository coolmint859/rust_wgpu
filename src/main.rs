mod graphics;
use graphics::state::State;

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ ActiveEventLoop, EventLoop, ControlFlow },
    keyboard::{ KeyCode, PhysicalKey },
    window::{ Window, WindowId },
};

pub struct App {
    state: Option<State>,
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window_attributes = Window::default_attributes().with_title("WGPU");
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            let state = pollster::block_on(State::new(window.clone()));

            self.state = Some(state);
        }
    }

    // fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {

    // }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let renderer = state.begin_frame();
                state.end_frame(renderer).unwrap();
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: key_state,
                    ..
                },
                ..
            } => {
                match (code, key_state.is_pressed()) {
                    (KeyCode::Escape, true) => event_loop.exit(),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    let mut app = App { state: None };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
