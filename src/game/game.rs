#![allow(dead_code)]

use glam::Vec3;

const PI: f32 = 3.1415;

use crate::graphics::{
    camera::{Camera, Camera2D}, entity::EntityInstances, init_state::StateInit, mesh::Mesh, presets::{MaterialPreset, RenderPipeline}, renderer::Renderer, shape_factory::Shape2D, traits::AppState, transform::Transform
};

pub struct Game {
    blue_devils: EntityInstances,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");
        let mut shape_factory = Shape2D::new();

        let mut transforms = Vec::new();

        let row_count = 50;
        for i in 0..row_count {
            let x = (2.0 * (i as f32) - (row_count as f32) + 1.0) / (row_count as f32);
            for j in 0..row_count {
                let y = (2.0 * (j as f32) - (row_count as f32) + 1.0) / (row_count as f32);
                
                transforms.push(
                    Transform::default()
                    .with_position(Vec3 { x, y, z: 0.0})
                    .with_scale(Vec3 { x: 0.01, y: 0.01, z: 0.1 })
                );
            }
        }

        let pipeline = RenderPipeline::TexturedSpriteInstanced.get();
        let blue_devils = EntityInstances { 
            mesh: Mesh::new("happy_tree", shape_factory.square(pipeline.primary_vertex_layouts())), 
            material: MaterialPreset::TexturedSprite("./assets/happy-tree.png").with_label("happy_tree"), 
            pipeline, 
            transforms
        };

        Self { blue_devils, camera }
    }
}

impl AppState for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, _dt: f32, _et: f32) {
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        renderer.draw_instances(&mut self.blue_devils);
    }
} 