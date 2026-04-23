#![allow(dead_code)]

const PI: f32 = 3.1415;

use glam::Vec3;
use rand::random;

use crate::{game::particle::{ParticleConfig, ParticleSystem, Variance}, graphics::{
    camera::{Camera, Camera2D}, init_state::StateInit, renderer::Renderer, traits::AppState,
}};

pub struct Game {
    particles: ParticleSystem,
    camera: Camera2D,
    emit_reset_time: f32,
    curr_emit_time: f32,
}

impl Game {
    pub fn new() -> Self {
        let camera = Camera2D::new("camera-2d");
        
        let particles = ParticleSystem::new( {
            ParticleConfig {
                num_particles: 1000,
                emit_center: Vec3 { x: 0.4, y: 0.5, z: 0.0},
                size: Variance { mean: 0.02, std_dev: 0.001 },
                speed: Variance { mean: 0.5, std_dev: 0.2 },
                lifespan: Variance { mean: 2.0, std_dev: 0.2 },
                rotation: Variance { mean: 0.3, std_dev: 0.001 },
                spin: Variance { mean: 5.0, std_dev: 2.0 },
                texture_path: "./assets/fire.png",
                is_one_shot: false
            }
        });

        Self { particles, camera, emit_reset_time: 3.0, curr_emit_time: 0.0 }
    }
}

impl AppState for Game {
    fn init(&mut self, _state_init: &mut StateInit) {

    }

    fn process_input(&mut self, _dt: f32, _et: f32) {
        
    }

    fn update(&mut self, dt: f32, _et: f32) {
        self.particles.update(dt);

        if self.curr_emit_time >= self.emit_reset_time {
            let x = random::<f32>() - 0.5;
            let y = random::<f32>() - 0.5;
            self.particles.set_emit_center(Vec3 { x, y, z: 0.0 });
            self.curr_emit_time = 0.0;
        } else {
            self.curr_emit_time += dt;
        }
    }

    fn render(&mut self, renderer: &mut Renderer, aspect: f32) {
        self.camera.set_aspect_ratio(aspect);

        // renderer.set_bg_color(0.392, 0.584, 0.929);
        renderer.set_camera(&mut self.camera);

        self.particles.render(renderer);
    }
} 