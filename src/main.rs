mod graphics;
use graphics::wpgu_context::WgpuContext;
use graphics::renderer::Renderer;
use graphics::traits::AppState;

use std::sync::Arc;
use std::time;

mod game;
use game::game::Game;

use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::*, event_loop::{ ActiveEventLoop, ControlFlow, EventLoop }, keyboard::{ KeyCode, PhysicalKey }, window::{ Window, WindowAttributes, WindowId }
};

use crate::graphics::{camera::{Camera, Camera2D}, init_state::StateInit, tracker::ResourceTracker};

pub struct App<T> {
    app_state: T,
    wgpu_ctx: Option<WgpuContext>,
    default_cam: Camera2D,
    prev_time: time::Instant,
    elapsed_time: f32,
    aspect_ratio: f32,
    attributes: WindowAttributes,
    reader_tracker: Option<ResourceTracker>
}

impl<T: AppState> App<T> {
    pub fn new(app_state: T, attributes: WindowAttributes) -> Self {
        Self { 
            default_cam: Camera2D::new("default-camera"),
            wgpu_ctx: None, 
            prev_time: time::Instant::now(),
            elapsed_time: 0.0, 
            app_state,
            aspect_ratio: 1.0,
            attributes,
            reader_tracker: Some(ResourceTracker::new())
        }
    }
}

impl<T: AppState> ApplicationHandler for App<T> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.wgpu_ctx.is_none() {
            let window = Arc::new(event_loop.create_window(self.attributes.clone()).unwrap());
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

        self.elapsed_time += dt;

        // println!("ET: {}", self.elapsed_time);

        self.app_state.process_input(dt, self.elapsed_time);
        self.app_state.update(dt, self.elapsed_time);

        let reader_tracker = self.reader_tracker.take().expect("Tracker missing!");
        self.reader_tracker = Some(wgpu_ctx.swap_trackers(reader_tracker));
        
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
                self.aspect_ratio = size.width as f32 / size.height as f32;
            },
            WindowEvent::RedrawRequested => {
                self.default_cam.set_aspect_ratio(self.aspect_ratio);

                let reader_tracker = self.reader_tracker.take().expect("Tracker Missing!");
                let mut renderer = Renderer::new(reader_tracker, self.elapsed_time);
                renderer.set_camera(&mut self.default_cam);

                self.app_state.render(&mut renderer, self.aspect_ratio);

                self.reader_tracker = Some(renderer.take_tracker());
                wgpu_ctx.create_resources(renderer.create_cmds());
                wgpu_ctx.update_resources(renderer.update_cmds());
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
    let attributes = Window::default_attributes()
        .with_inner_size(Size::Physical(
            PhysicalSize { width: 1920, height: 1080 }
        ))
        .with_title("WGPU Renderer");

    let mut app = App::new(game, attributes);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
}
