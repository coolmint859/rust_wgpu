mod graphics;
use graphics::context::WgpuContext;
use graphics::transient::Renderer;
use graphics::traits::AppState;

use std::sync::Arc;
use std::time;

mod game;
use game::game::Game;

use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ ActiveEventLoop, EventLoop, ControlFlow },
    keyboard::{ KeyCode, PhysicalKey },
    window::{ Window, WindowId },
};

use crate::graphics::transient::StateInit;

pub struct App<T> {
    app_state: T,
    wgpu_ctx: Option<WgpuContext>,
    prev_time: time::Instant,
}

impl<T: AppState> App<T> {
    pub fn new(app_state: T) -> Self {
        let start_time = time::Instant::now();
        Self { wgpu_ctx: None , prev_time: start_time, app_state }
    }
}

impl<T: AppState> ApplicationHandler for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.wgpu_ctx.is_none() {
            let window_attributes = Window::default_attributes().with_title("WGPU");
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            let mut wgpu_ctx = pollster::block_on(WgpuContext::new(window.clone()));

            let mut init_state = StateInit::new();
            self.app_state.init(&mut init_state);
            wgpu_ctx.init_resources(init_state);

            self.wgpu_ctx = Some(wgpu_ctx);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let wgpu_ctx = match &mut self.wgpu_ctx {
            Some(wgpu_ctx) => wgpu_ctx,
            None => return
        };

        let curr_time = time::Instant::now();
        let dt = (curr_time - self.prev_time).as_secs_f32();
        self.prev_time = curr_time;

        println!("FPS: {}", 1.0/dt);

        self.app_state.process_input();
        self.app_state.update(dt);

        wgpu_ctx.update_state();
        wgpu_ctx.prepare_next_frame();
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
                let mut renderer = Renderer::new();
                self.app_state.render(&mut renderer);
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

    let game = Game::new();
    let mut app = App::new(game);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
