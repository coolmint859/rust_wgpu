#![allow(dead_code)]

use glam::{Quat, Vec3};

const PI: f32 = 3.1415;

use crate::graphics::{
    camera::{Camera, Camera2D}, entity::Entity, init_state::StateInit, mesh::Mesh, presets::{MaterialPreset, RenderPipeline}, renderer::Renderer, shape_factory::Shape2D, traits::AppState, transform::Transform
};

pub struct Game {
    // shapes: Vec<Mesh<ColoredSprite>>,
    square1: Entity,
    square2: Entity,
    square3: Entity,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");
        let mut shape_factory = Shape2D::new();

        let tex_pipeline = RenderPipeline::TexturedSprite.get();
        let color_pipeline = RenderPipeline::ColoredSprite.get();

        let square1 = Entity { 
            mesh: Mesh::new("happy_tree", shape_factory.square(tex_pipeline.vertex_layout())), 
            transform: Transform::new(Vec3 {x: -0.6, y: 0.3, z: 0.0 }, Quat::IDENTITY, Vec3 { x: 0.3, y: 0.3, z: 1.0 }), 
            material: MaterialPreset::TexturedSprite("./assets/happy-tree.png").with_label("happy_tree"),
            pipeline: tex_pipeline.clone()
        };

        let square2 = Entity { 
            mesh: Mesh::new("blue_devils", shape_factory.square(tex_pipeline.vertex_layout())),
            transform: Transform::new(Vec3 {x: 0.6, y: 0.3, z: 0.0 }, Quat::IDENTITY, Vec3 { x: 0.3, y: 0.3, z: 1.0 }),
            material: MaterialPreset::TexturedSprite("./assets/BlueDevilsLogo.png").with_label("blue_devils"),
            pipeline: tex_pipeline.clone()
        };

        let square3 = Entity {
            mesh: Mesh::new("blue_square", shape_factory.square(color_pipeline.vertex_layout())),
            transform: Transform::new(Vec3 {x: 0.0, y: -0.3, z: 0.0 }, Quat::IDENTITY, Vec3 { x: 0.3, y: 0.3, z: 1.0 }),
            material: MaterialPreset::ColoredSprite([0.0, 0.0, 1.0, 1.0]).with_label("bluey"),
            pipeline: color_pipeline
        };

        Self { square1, square2, square3, camera }
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
        renderer.draw(&mut self.square3);
    }
} 