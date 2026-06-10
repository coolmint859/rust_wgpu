#![allow(dead_code)]

const PI: f32 = 3.1415;

use glam::{Quat, Vec3, Vec4};

use super::{
    animation::{AnimationController, CyclicAnimator, FadeAnimation, FadeMode, TextureAnimation},
    particle::{ParticleConfig, ParticleEmitter},
    particle_systems::*,
};

use crate::graphics::{
    camera::{Camera, Camera2D}, font::Font, geometry::{Geometry, PositionAttribute, UVAttribute}, init_state::StateInit, instance::{InstanceGroup, TintAttribute, TransformAttribute, UVBoundsAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, primitive::{Primitive, RenderInfo}, renderer::{Renderer, TextOptions}, shape_factory::Shape2D, traits::{Driver, GameSystem}, transform::Transform
};

pub struct Game {
    flags: Primitive,
    flag_animator: CyclicAnimator,
    particles: ParticleEmitter<PointSpawner2D>,
    camera: Camera2D,
    font: Font,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");

        let transforms = vec![
            Transform::new(Vec3 { x: -0.5, y: 0.5, z: 0.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
            Transform::new(Vec3 { x: 0.5, y: 0.5, z: 0.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
            Transform::new(Vec3 { x: 0.5, y: -0.5, z: 0.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
            Transform::new(Vec3 { x: -0.5, y: -0.5, z: 0.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
        ];
        let uv_bounds = vec![Vec4::new(0.0, 0.0, 1.0, 1.0); transforms.len()];
        let colors = vec![Vec4::new(1.0, 1.0, 0.0, 1.0); transforms.len()];

        let instances = InstanceGroup::new(transforms.len(), transforms.len())
            .with_attribute(TransformAttribute, transforms)
            .with_attribute(TintAttribute("color", 10), colors)
            .with_attribute(UVBoundsAttribute, uv_bounds);

        let flags = Primitive::from_group(
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

        let flag_animator = CyclicAnimator::new(vec![0.15, 0.15, 0.15])
            .with_animation(TextureAnimation::new(3, 1))
            .with_animation(FadeAnimation::new(FadeMode::Sinusoidal(0.0), 1.5));

        let lifecycle = PointSpawner2D::new(
            Variance { mean: 3.0, std_dev: 0.08 },
            Variance { mean: 0.1, std_dev: 0.01 },
            Vec3::new(0.0, 0.0, 1.0),
        );

        // let lifecycle = PlaneSpawner {
        //     emit_width: 5.0,
        //     sky_y: 1.2,
        //     floor_y: -1.2,
        //     max_size: 0.01,
        // };

        let particle_config = ParticleConfig {
            total_particles: 5000,
            emit_cap: 80,
            is_one_shot: false
        };

        // let particles = ParticleEmitter::colored(Vec4::new(0.0, 129.0/255.0, 185.0/255.0, 1.0), particle_config, lifecycle)
        //     .with_behavior(WeatherForceBehavior {
        //         gravity: -9.8,
        //         wind_force: 1.0,
        //         terminal_velocity: 3.0,
        //         max_delay: 1.5
        //     });

        let particles = ParticleEmitter::textured("./assets/pride_flag.jpg", particle_config, lifecycle)
            .with_behavior(RadialKinematicsBehavior::new(
                Variance { mean: 0.3, std_dev: 0.05 },
                Variance { mean: 1.0, std_dev: 0.1 }
            ))
            .with_behavior(FadeBehavior::new(FadeMode::Decrease));

        let font = Font::new("./assets/Monopack.ttf");

        Self {particles, flags, flag_animator, camera, font }
    }
}

impl Driver for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, dt: f32, et: f32) {
        self.flag_animator.animate(&mut self.flags, dt, et);
        self.particles.update(dt, et);
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        // renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_bg_color(0.0, 0.05, 0.2);
        // renderer.set_bg_color(0.8, 0.8, 0.8);
        renderer.set_camera(&mut self.camera);

        renderer.draw(&mut self.flags);

        renderer.draw_text(
            "The quick brown fox jumped over the lazy dog.",
            &self.font,
            TextOptions {
                pos: Vec3::new(-1.25, 0.0, 0.0),
                width: 2.5,
                text_color: Vec4::ONE,
                // outline_color: None
                outline_color: Some(Vec4::new(0.0, 0.0, 0.0, 1.0))
            }
        );

        // self.particles.render(renderer);
    }
}