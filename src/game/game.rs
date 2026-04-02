#![allow(dead_code)]
use glam::Vec3;

use crate::graphics::{
    mesh::Mesh, 
    presets::{Pipeline, Shape2D}, 
    traits::AppState, 
    init_state::StateInit,
    renderer::Renderer,
    material::ColoredSprite,
};

pub struct Game {
    // shapes: Vec<Mesh<ColoredSprite>>,
    square: Mesh<ColoredSprite>
}

impl Game {
    pub fn new() -> Self {
        let mut shape_factory = Shape2D::new();

        let square = Mesh::new(
            "cube",
            shape_factory.square(),
            ColoredSprite::new([0.0, 1.0, 0.0, 1.0]),
            Pipeline::ColoredSprite.get()
        );

        Self { 
            square
        }
    }
}

impl AppState for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32) {
        
    }

    fn update(&mut self, dt: f32) {
        self.square.transform.translate(Vec3::new(0.1*dt, 0.0, 0.0));
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.set_bg_color(0.392, 0.584, 0.929);

        renderer.draw(&self.square);

        // for shape in &self.shapes {
        //     renderer.draw(shape);
        // }
    }
}