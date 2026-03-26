#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{
    mesh::Mesh, presets::Pipeline, presets::Shape2D, traits::AppState, transient::{RenderCommand, Renderer, StateInit}
};

pub struct Game {
    shapes: Vec<Arc<Mesh>>,
}

impl Game {
    pub fn new() -> Self {
        let mut shape_factory = Shape2D::new();

        let triangle = Mesh::new(
            "triangle",
            shape_factory.triangle()
        );

        let square = Mesh::new(
            "cube",
            shape_factory.square(),
        );

        Self { 
            shapes: vec![Arc::new(triangle), Arc::new(square)]
        }
    }
}

impl AppState for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self) {
        
    }

    fn update(&mut self, _dt: f32) {
        
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.set_bg_color(0.392, 0.584, 0.929);

        for shape in self.shapes.iter() {
            renderer.draw(
            RenderCommand::Mesh(
                Arc::clone(shape), 
                Pipeline::ColoredSprite.get()
            ))
        }
    }
}