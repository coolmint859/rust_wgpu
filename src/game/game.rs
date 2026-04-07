#![allow(dead_code)]
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
            ColoredSprite::new([0.0, 0.3, 0.5, 1.0]),
            Pipeline::ColoredSprite.get()
        );

        let camera = Camera2D::new("main-cam");

        Self { square, camera }
    }
}

impl AppState for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, _dt: f32, et: f32) {
        let r = ((et*3.0).sin() + 1.0) / 2.0;
        let g = ((et*2.0).sin() + 1.0) / 2.0;
        let b = ((et*1.0).sin() + 1.0) / 2.0;
        self.square.material.set_color([r, g, b, 1.0]);

        let jr = 12.0;
        let jitter = (rand::random::<f32>() / jr) - (1.0/jr);
        self.square.transform.set_rotation_euler(0.0, 0.0, jitter);

        let zoom = ((et*2.0).sin() / 4.0) + 0.5;
        self.camera.set_zoom(zoom);
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        // renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        renderer.draw(&mut self.square);
    }
} 