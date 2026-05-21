#![allow(dead_code)]

const PI: f32 = 3.1415;

use glam::{Quat, Vec3, Vec4};

use crate::{game::particle::{ParticleConfig, ParticleEmitter2D, Variance}, graphics::{
    animation::{SyncedAnimatedSprite, SyncedAnimatedSpriteConfig}, camera::{Camera, Camera2D}, entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, init_state::StateInit, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::{AnimationController, Driver}, transform::Transform
}};

pub struct Game {
    particles: ParticleEmitter2D,
    flag: SyncedAnimatedSprite,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");
        
        let flag = SyncedAnimatedSprite::from_config(SyncedAnimatedSpriteConfig {
            path: "./assets/flag.png",
            frame_times: vec![0.05, 0.20, 0.10],
            transforms: vec![
                Transform::new(Vec3 { x: -0.5, y: 0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
                Transform::new(Vec3 { x: 0.5, y: 0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
                Transform::new(Vec3 { x: 0.5, y: -0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
                Transform::new(Vec3 { x: -0.5, y: -0.5, z: 1.0}, Quat::IDENTITY, Vec3 { x: 0.15, y: 0.15, z: 1.0}),
                ],
            color: Vec4::new(1.0, 1.0, 0.0, 1.0),
        });

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

        Self { particles, flag, camera, }
    }
}

impl Driver for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, dt: f32, _et: f32) {
        self.flag.update(dt);
        // self.particles.update(dt);
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        // renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        self.flag.render(renderer);

        // self.particles.render(renderer);
    }
} 