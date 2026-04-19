#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use glam::Vec3;

const PI: f32 = 3.1415;

use crate::graphics::{
    camera::{Camera, Camera2D}, init_state::StateInit, material::{ColorComponent, Material}, mesh::Mesh, presets::{RenderPipeline, Shape2D}, renderer::{Entity, Renderer}, traits::AppState, transform::Transform
};

pub struct Game {
    // shapes: Vec<Mesh<ColoredSprite>>,
    squares: Vec<Entity>,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");
        let mut shape_factory = Shape2D::new();
        let mut squares: Vec<Entity> = Vec::new();

        let mut map = HashMap::new();
        map.insert("color".to_string(), 0);

        let mut material = Material::new("colored-sprite", map);
        match material.add_component(ColorComponent::new("color", [1.0, 0.0, 0.0, 1.0])) {
            Err(msg) => panic!("{}", msg),
            _ => {}
        }
        let material = Arc::new(material);

        let n = 6;
        for i in 0..n {
            let square = Mesh::new(
                "square",
                shape_factory.square()
            );

            let b = 0.8;
            let x = b * (2.0 * i as f32 / n as f32 - 1.0);
            
            let mut transform = Transform::default();
            transform.translate(Vec3::new(x, 0.0, 0.0));
            transform.scale(Vec3::new(0.05, 0.05, 1.0));

            squares.push(Entity { 
                mesh: square, 
                transform,
                material: material.clone(), 
                pipeline: RenderPipeline::ColoredSprite.get() 
            });
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
        let mut i = 0.0;
        for square in &mut self.squares {
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