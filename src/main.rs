mod graphics;
use graphics::renderer::Renderer;
use graphics::traits::Driver;

use std::sync::Arc;
use std::time;

mod game;
use game::game::Game;

use winit::{
    application::ApplicationHandler, dpi::{PhysicalSize, Size}, event::*, event_loop::{ ActiveEventLoop, ControlFlow, EventLoop }, keyboard::{ KeyCode, PhysicalKey }, window::{ Window, WindowAttributes, WindowId }
};

use crate::graphics::{camera::{Camera, Camera2D}, init_state::StateInit};

pub struct App<D> {
    driver: D,
    renderer: Option<Renderer>,
    default_cam: Camera2D,
    prev_time: time::Instant,
    elapsed_time: f32,
    aspect_ratio: f32,
    attributes: WindowAttributes,
}

impl<D: Driver> App<D> {
    pub fn new(driver: D, attributes: WindowAttributes) -> Self {
        Self { 
            default_cam: Camera2D::new("default-camera"),
            renderer: None, 
            prev_time: time::Instant::now(),
            elapsed_time: 0.0, 
            driver,
            aspect_ratio: 1.0,
            attributes,
        }
    }
}

impl<D: Driver> ApplicationHandler for App<D> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_none() {
            let window = Arc::new(event_loop.create_window(self.attributes.clone()).unwrap());
            let mut renderer = pollster::block_on(Renderer::new(window.clone()));
            renderer.set_camera(&mut self.default_cam);

            let mut init_state = StateInit::new();
            self.driver.init(&mut init_state);
            renderer.init_resources(init_state);

            self.renderer = Some(renderer);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let renderer = match &mut self.renderer {
            Some(renderer) => renderer,
            None => return
        };

        let curr_time = time::Instant::now();
        let dt = (curr_time - self.prev_time).as_secs_f32();
        self.prev_time = curr_time;

        self.elapsed_time += dt;

        // println!("ET: {}", self.elapsed_time);

        self.driver.process_input(dt, self.elapsed_time);
        self.driver.update(dt, self.elapsed_time);
        
        renderer.begin_frame(self.elapsed_time);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let mut renderer = match &mut self.renderer {
            Some(renderer) => renderer,
            None => return
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                renderer.resize(size.width, size.height);
                self.aspect_ratio = size.width as f32 / size.height as f32;
            },
            WindowEvent::RedrawRequested => {
                self.default_cam.set_aspect_ratio(self.aspect_ratio);
                self.driver.render(&mut renderer, self.aspect_ratio);
                renderer.end_frame();
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
