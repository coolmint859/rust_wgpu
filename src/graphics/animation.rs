#![allow(dead_code)]

use glam::Vec4;

use crate::graphics::{entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, instance::{InstanceGroup, TintAttribute, TransformAttribute, UVBoundsAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::AnimationController, transform::Transform};

pub struct SyncedAnimatedSpriteConfig {
    pub path: &'static str, // replace with dedicated struct
    pub frame_times: Vec<f32>,
    pub transforms: Vec<Transform>,
    pub color: glam::Vec4 
}

/// An animated sprite where instance animations are updated (synced) together.
pub struct SyncedAnimatedSprite  {
    sprites: Entity,
    frame_times: Vec<f32>,
    frames: SpriteSheet,
    current_frame: usize,
    timer: f32,
}

impl SyncedAnimatedSprite {
    pub fn from_config(config: SyncedAnimatedSpriteConfig) -> Self {
        let geometry = Geometry::new(Shape2D::new().square())
            .with_attribute(PositionAttribute)
            .with_attribute(UVAttribute);

        let mut uv_bounds = Vec::new();
        let mut colors = Vec::new();
        for _ in 0..config.transforms.len() {
            uv_bounds.push(glam::Vec4::new(0.0, 0.0, 1.0, 1.0));
            colors.push(config.color.clone())
        }

        let instances = InstanceGroup::new(config.transforms.len(), config.transforms.len())
            .with_attribute(TransformAttribute, config.transforms)
            .with_attribute(TintAttribute, colors)
            .with_attribute(UVBoundsAttribute, uv_bounds);

        let sprites = Entity::from_group(
            config.path, 
            geometry, 
            MaterialPreset::TexturedSprite(config.path.to_string()).with_label("animated-sprite"), 
            instances, 
            RenderInfo { 
                shader_path: ShaderSpecPreset::AnimatedSprite.path(), 
                pipeline: RenderPipeline::AnimatedSprite.get(), 
            }
        );

        Self {
            sprites,
            frames: SpriteSheet::new(config.frame_times.len(), 1),
            frame_times: config.frame_times,
            current_frame: 0,
            timer: 0.0,
        }
    }
}

impl AnimationController for SyncedAnimatedSprite {
    fn update(&mut self, dt: f32) {
        self.timer += dt;

        if let Some(uv_bounds) = self.sprites.instances.get_attribute_mut::<glam::Vec4>(UVBoundsAttribute) {;
            if self.timer < self.frame_times[self.current_frame] { return; }
            self.current_frame = (self.current_frame + 1) % self.frame_times.len();

            for bounds in uv_bounds {
                bounds.set(*self.frames.get(self.current_frame));
            }

            self.timer = 0.0;
        }

        // println!("timer: {:?}", self.timer);
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw(&mut self.sprites);
    }
}

pub struct SpriteSheet {
    pub frames: Vec<Vec4>,
    pub current_frame: usize,
}

impl SpriteSheet {
    fn new(cols: usize, rows: usize) -> Self {
        let mut frames = Vec::with_capacity(cols * rows);

        let scale_x = 1.0 / cols as f32;
        let scale_y = 1.0 / rows as f32;

        for r in 0..rows {
            for c in 0..cols {
                let offset_x = (c as f32) * scale_x;
                let offset_y = (r as f32) * scale_y;

                frames.push(Vec4::new(offset_x, offset_y, scale_x, scale_y));
            }
        }

        Self { frames, current_frame: 0 }
    }

    fn get(&self, idx: usize) -> &Vec4 {
        self.frames.get(idx).unwrap()
    }
}

// impl Iterator for SpriteSheet {
//     type Item = Vec4;

//     fn next(&mut self) -> Option<Self::Item> {
        
//     }
// }