#![allow(dead_code)]

const PI: f32 = 3.1415;

use glam::Vec3;

use crate::{game::particle::{ParticleConfig, ParticleSystem, Variance}, graphics::{
    camera::{Camera, Camera2D}, entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, init_state::StateInit, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::Driver, transform::Transform
}};

pub struct Game {
    particles: ParticleSystem,
    blue_devils: Entity,
    camera: Camera2D,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");
        
        let geometry = Geometry::new(Shape2D::new().square())
            .with_attribute(PositionAttribute)
            .with_attribute(UVAttribute);

        let blue_devils = Entity::new(
            "blue-devils",
            geometry,
            MaterialPreset::TexturedSprite("./assets/BlueDevilsLogo.png".to_string()).with_label("blue-devils"),
            Transform::identity(),
            RenderInfo {
                shader_path: ShaderSpecPreset::TexturedSprite.path(),
                pipeline: RenderPipeline::TexturedSprite.get()
            }
        );

        let particles = ParticleSystem::new(ParticleConfig {
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

        Self { particles, blue_devils, camera, }
    }
}

impl Driver for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, dt: f32, et: f32) {
        self.blue_devils.first_mut().transform_mut().set_x(et-1.0);
        self.particles.update(dt);
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        // renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        renderer.draw(&mut self.blue_devils);

        self.particles.render(renderer);
    }
} 