#![allow(dead_code)]

const PI: f32 = 3.1415;

use glam::{Quat, Vec3, Vec4};

use crate::{game::particle::{ParticleConfig, ParticleEmitter2D, Variance}, graphics::{
    animation::{AnimationController, CyclicAnimator, FadeAnimation, FadeMode, TextureAnimation}, camera::{Camera, Camera2D}, entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, init_state::StateInit, instance::{InstanceGroup, TintAttribute, TransformAttribute, UVBoundsAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::Driver, transform::Transform
}};

pub struct Game {
    particles: ParticleEmitter2D,
    flags: Entity,
    animator: CyclicAnimator,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");

        let transforms = vec![
            Transform::new(Vec3 { x: -0.5, y: 0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
            Transform::new(Vec3 { x: 0.5, y: 0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
            Transform::new(Vec3 { x: 0.5, y: -0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
            Transform::new(Vec3 { x: -0.5, y: -0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
        ];
        let uv_bounds = vec![Vec4::new(0.0, 0.0, 1.0, 1.0); transforms.len()];
        let colors = vec![Vec4::new(1.0, 1.0, 0.0, 1.0); transforms.len()];

        let instances = InstanceGroup::new(transforms.len(), transforms.len())
            .with_attribute(TransformAttribute, transforms)
            .with_attribute(TintAttribute, colors)
            .with_attribute(UVBoundsAttribute, uv_bounds);

        let flags = Entity::from_group(
            "flags", 
            Geometry::new(Shape2D::new().square())
                .with_attribute(PositionAttribute)
                .with_attribute(UVAttribute),
            MaterialPreset::TexturedSprite("./assets/flag.png".to_string()).with_label("animated-sprite"), 
            instances, 
            RenderInfo { 
                shader_path: ShaderSpecPreset::AnimatedSprite.path(), 
                pipeline: RenderPipeline::AnimatedSprite.get(), 
            }
        );

        let animator = CyclicAnimator::new(vec![0.15, 0.15, 0.15])
            .with_animation(TextureAnimation::new(3, 1))
            .with_animation(FadeAnimation::new(FadeMode::Sinusoidal(0.0), 1.5));

        let particles = ParticleEmitter2D::new(ParticleConfig {
            total_particles: 5000,
            spawn_cap: 500,
            emit_center: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
            size: Variance { mean: 0.02, std_dev: 0.001 },
            speed: Variance { mean: 0.5, std_dev: 0.2 },
            lifespan: Variance { mean: 2.00, std_dev: 0.2 },
            rotation: Variance { mean: 0.3, std_dev: 0.001 },
            spin: Variance { mean: 5.0, std_dev: 2.0 },
            texture_path: "./assets/fire.png",
            is_one_shot: false
        });

        Self { particles, flags, animator, camera }
    }
}

impl Driver for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, dt: f32, et: f32) {
        self.animator.animate(&mut self.flags, dt, et);
        // self.particles.update(dt);
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        // renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        renderer.draw(&mut self.flags);

        // self.particles.render(renderer);
    }
} 