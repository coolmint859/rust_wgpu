#![allow(dead_code)]
use glam::Vec3;

use crate::graphics::{
    camera::{Camera, Camera2D}, init_state::StateInit, material::ColoredSprite, mesh::Mesh, presets::{Pipeline, Shape2D}, renderer::Renderer, traits::AppState
};

pub struct Game {
    // shapes: Vec<Mesh<ColoredSprite>>,
    square: Mesh<ColoredSprite>,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let mut shape_factory = Shape2D::new();

        let square = Mesh::new(
            shape_factory.square(),
            ColoredSprite::new([1.0, 0.0, 0.0, 1.0]),
            Pipeline::ColoredSprite.get()
        );

        let camera = Camera2D::default("main-cam");

        Self { square, camera }
    }
}

impl AppState for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32) {
        
    }

    fn update(&mut self, dt: f32) {
        // self.square.transform.translate(Vec3::new(0.1*dt, 0.1*dt, 0.0));
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        renderer.draw(&mut self.square);
    }
} 