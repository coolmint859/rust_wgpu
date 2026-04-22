#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use glam::Vec3;

const PI: f32 = 3.1415;

use crate::graphics::{
    camera::{Camera, Camera2D}, entity::Entity, init_state::StateInit, material::{Material, SamplerComponent, TextureComponent}, mesh::Mesh, presets::{RenderPipeline, Shape2D, TextureSampler}, renderer::Renderer, traits::AppState, transform::Transform
};

pub struct Game {
    // shapes: Vec<Mesh<ColoredSprite>>,
    square1: Entity,
    square2: Entity,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");
        let mut shape_factory = Shape2D::new();

        // square 1
        let mut map = HashMap::new();
        map.insert("happy_tree_tex".to_string(), 0);
        map.insert(TextureSampler::NearestClampToEdge.as_key(), 1);

        let mut material = Material::new("colored-sprite", map.clone());
        match material.add_component(TextureComponent::new("happy_tree_tex", "./assets/happy-tree.png")) {
            Err(msg) => panic!("{}", msg),
            _ => {}
        }
        match material.add_component(SamplerComponent::new(TextureSampler::NearestClampToEdge).with_bind_slot(1)) {
            Err(msg) => panic!("{}", msg),
            _ => {}
        }
        let material = Arc::new(material);

        let square1 = Entity { 
            mesh: Mesh::new("happy_tree", shape_factory.square()), 
            transform: Transform::default().with_position(Vec3 {x: -0.6, y: 0.0, z: 0.0 }), 
            material, 
            pipeline: RenderPipeline::TexturedSprite.get() 
        };

        // square 2
        let mut map = HashMap::new();
        map.insert("blue_devils_tex".to_string(), 0);
        map.insert(TextureSampler::NearestClampToEdge.as_key(), 1);

        let mut material = Material::new("colored-sprite", map);
        match material.add_component(TextureComponent::new("blue_devils_tex", "./assets/BlueDevilsLogo.png")) {
            Err(msg) => panic!("{}", msg),
            _ => {}
        }
        match material.add_component(SamplerComponent::new(TextureSampler::NearestClampToEdge).with_bind_slot(1)) {
            Err(msg) => panic!("{}", msg),
            _ => {}
        }
        let material = Arc::new(material);

        let square2 = Entity { 
            mesh: Mesh::new("blue_devils", shape_factory.square()),
            transform: Transform::default().with_position(Vec3 {x: 0.6, y: 0.0, z: 0.0 }),
            material, 
            pipeline: RenderPipeline::TexturedSprite.get() 
        };

        Self { square1, square2, camera }
    }
}

impl AppState for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, _dt: f32, _et: f32) {
        // self.square1.transform.set_y(et.sin() / 2.0);
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        renderer.draw(&mut self.square1);
        renderer.draw(&mut self.square2);
    }
} 