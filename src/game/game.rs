#![allow(dead_code)]
use glam::Vec3;

const PI: f32 = 3.1415;

use crate::graphics::{
    camera::{Camera, Camera2D}, 
    init_state::StateInit, 
    material::ColoredSprite, 
    mesh::Mesh, 
    presets::{RenderPipeline, Shape2D}, 
    renderer::Renderer, 
    traits::AppState,
};

pub struct Game {
    // shapes: Vec<Mesh<ColoredSprite>>,
    squares: Vec<Mesh<ColoredSprite>>,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("main-cam");
        let mut shape_factory = Shape2D::new();
        let mut squares: Vec<Mesh<ColoredSprite>> = Vec::new();

        let n = 5;
        for i in 0..n {
            let mut square = Mesh::new(
                shape_factory.square(),
                ColoredSprite::new([0.0, 0.3, 0.5, 1.0]),
                RenderPipeline::ColoredSprite.get()
            );

            let b = 0.8;
            let x = b * (2.0 * i as f32 / n as f32 - 1.0);

            println!("{x}");
            square.transform.translate(Vec3::new(x, 0.0, 0.0));
            square.transform.scale(Vec3::new(0.05, 0.05, 1.0));

            squares.push(square);
        }

        Self { squares, camera }
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

        let mut i = 0.0;
        for square in &mut self.squares {
            square.material.set_color([r, g, b, 1.0]);

            let y = (et + i / (PI/2.0)).sin() / 2.0;
            square.transform.set_y(y);
            square.transform.set_rotation_euler(0.0, 0.0, et);
            i += 1.0;
        }
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        // renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        for square in &mut self.squares {
            renderer.draw(square);
        }
    }
} 