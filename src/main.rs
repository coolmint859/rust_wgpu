mod graphics;
use graphics::context::WgpuContext;
use graphics::renderer::Renderer;

use std::sync::Arc;
use std::time;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ ActiveEventLoop, EventLoop, ControlFlow },
    keyboard::{ KeyCode, PhysicalKey },
    window::{ Window, WindowId },
};

pub struct App {
    wgpu_ctx: Option<WgpuContext>,
    prev_time: time::Instant,
}

impl App {
    pub fn new() -> Self {
        let start_time = time::Instant::now();
        Self { wgpu_ctx: None , prev_time: start_time }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.wgpu_ctx.is_none() {
            let window_attributes = Window::default_attributes().with_title("WGPU");
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            let state = pollster::block_on(WgpuContext::new(window.clone()));

            self.wgpu_ctx = Some(state);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let curr_time = time::Instant::now();
        let dt = (curr_time - self.prev_time).as_secs_f32();
        self.prev_time = curr_time;


        println!("FPS: {}", 1.0/dt);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let wgpu_ctx = match &mut self.wgpu_ctx {
            Some(wgpu_ctx) => wgpu_ctx,
            None => return
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                wgpu_ctx.resize(size.width, size.height);
            },
            WindowEvent::RedrawRequested => {
                let renderer = Renderer::new();
                // self.game.render(&mut renderer);
                wgpu_ctx.render(renderer).unwrap();
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

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut app = App::new();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
