#![allow(dead_code)]

use crate::graphics::{entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, transform::Transform};

pub struct AnimatedSprite  {
    entities: Vec<Entity>,
    times: Vec<f32>,
    current_entity: usize,
    timer: f32,
    
    pub transform: Transform,
}

impl AnimatedSprite {
    pub fn new(images: Vec<&str>, times: Vec<f32>) -> Self {
        assert!(images.len() == times.len());

        let geometry_data = Shape2D::new().square();

        let mut entities = Vec::new();
        for image in images {
            let geometry = Geometry::new(geometry_data.clone())
                .with_attribute(PositionAttribute)
                .with_attribute(UVAttribute);

            let entity = Entity::new(
                "animated-sprite",
                geometry,
                MaterialPreset::TexturedSprite(image.to_string()).with_label("animated-sprite"),
                Transform::identity(),
                RenderInfo {
                    shader_path: ShaderSpecPreset::TexturedSprite.path(),
                    pipeline: RenderPipeline::TexturedSprite.get()
                }
            );

            entities.push(entity);
        }

        Self {
            entities,
            times,
            current_entity: 0,
            timer: 0.0,
            transform: Transform::identity()
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.timer += dt;

        if self.timer >= self.times[self.current_entity] {
            self.timer = 0.0;
            self.current_entity = (self.current_entity + 1) % self.entities.len()
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw(&mut self.entities[self.current_entity]);
    }
}