#![allow(dead_code)]
use std::f32::consts::PI;

use glam::{Quat, Vec3};
use rand::random;
use rand_distr::{Distribution, Normal};

use crate::graphics::{
    entity::EntityInstances,
    presets::{MaterialPreset, RenderPipeline}, 
    renderer::Renderer, 
    shape_factory::Shape2D, 
    transform::Transform
};

pub struct Variance {
    pub mean: f32,
    pub std_dev: f32,
}

/// Configuration struct for the particle system
pub struct ParticleConfig {
    /// the total number of particles to render
    pub total_particles: usize,
    /// the number of particles to emit per update
    pub spawn_cap: usize,
    /// the emission center of the particle
    pub emit_center: Vec3,
    /// the mean and std deviation of the particles' lifespans
    pub lifespan: Variance,
    /// the mean and std deviation of the particles' speeds
    pub speed: Variance,
    /// the mean and std deviation of the particles' size
    pub size: Variance,
    /// the mean and std deviation of the particles' initial rotation
    pub rotation: Variance,
    /// the mean and std deviation of the particles' spin (angular speed)
    pub spin: Variance,
    /// the path to the texture to render on the particles
    pub texture_path: &'static str,
    /// if true, only one burst of particles will occur, otherwise 'dead' particles
    /// will be reborn with all properties randomized.
    pub is_one_shot: bool,
}

/// Houses the distributions for particle behavior
struct ParticleDistributions {
    pub speed: Normal<f32>,
    pub lifespan: Normal<f32>,
    pub size: Normal<f32>,
    pub rotation: Normal<f32>,
    pub spin: Normal<f32>,
}

impl ParticleDistributions {
    pub fn new(config: &ParticleConfig) -> Self {
        Self {
            speed: Normal::new(config.speed.mean, config.speed.std_dev).unwrap(),
            lifespan: Normal::new(config.lifespan.mean, config.lifespan.std_dev).unwrap(),
            size: Normal::new(config.size.mean, config.size.std_dev).unwrap(),
            rotation: Normal::new(config.rotation.mean, config.rotation.std_dev).unwrap(),
            spin: Normal::new(config.spin.mean, config.spin.std_dev).unwrap(),
        }
    }
}

/// A 2D particle system using instanced rendering.
/// 
/// Particle behvaior is determined by normal distributions
pub struct ParticleSystem {
    /// the configuration of the particle system
    config: ParticleConfig,
    /// the distributions for particle behavior
    dist: ParticleDistributions,
    /// the entity instances for rendering
    instances: EntityInstances,
    /// particle velocities
    velocities: Vec<Vec3>,
    /// particle spins
    spins: Vec<f32>,
    /// particle lifetimes
    lifetimes: Vec<f32>,
    /// particle lifespans
    lifespans: Vec<f32>,
}

impl ParticleSystem {
    pub fn new(config: ParticleConfig) -> Self {
        let pipeline = RenderPipeline::TexturedSpriteInstanced.get();
        let instances = EntityInstances {
            geometry: Shape2D::new().square(pipeline.primary_vertex_layouts()),
            material: MaterialPreset::TexturedSprite(config.texture_path).with_label("particle"),
            transforms: Vec::with_capacity(config.total_particles),
            pipeline,
        };

        let start_particles = config.spawn_cap;
        let dist = ParticleDistributions::new(&config);

        let mut system = Self {
            config,
            dist,
            instances,
            velocities: Vec::new(),
            spins: Vec::new(),
            lifespans: Vec::new(),
            lifetimes: Vec::new()
        };

        system.spawn_particles(start_particles);

        system
    }

    /// spawn as many particles as possible
    pub fn burst(&mut self) {
        let count = self.config.total_particles - self.instances.transforms.len();
        self.spawn_particles(count);
    }

    /// Get the number of remaining particles left - this only changes if is_one_shot is true
    pub fn remaining_particles(&self) -> usize {
        self.instances.transforms.len()
    }

    /// Set the emit center for this particle system. Only reset particles will use this if is_one_shot is false.
    pub fn set_emit_center(&mut self, center: Vec3) {
        self.config.emit_center = center;
    }

    /// Update the particles in this particle system.
    pub fn update(&mut self, dt: f32) {
        if self.instances.transforms.len() == 0 { return; }

        // spawn new particles if continuous
        if !self.config.is_one_shot {
            let current_alive = self.instances.transforms.len();
            let available_space = self.config.total_particles.saturating_sub(current_alive);
            let to_spawn = self.config.spawn_cap.min(available_space);

            self.spawn_particles(to_spawn);
        }

        // update all existing particles
        let mut i = 0;
        while i < self.lifetimes.len() {
            self.lifetimes[i] += dt;

            if self.lifetimes[i] >= self.lifespans[i] {
                self.lifetimes.swap_remove(i);
                self.lifespans.swap_remove(i);
                self.velocities.swap_remove(i);
                self.spins.swap_remove(i);
                self.instances.transforms.swap_remove(i);
            } else {
                self.instances.transforms[i].translate(self.velocities[i] * dt);
                self.instances.transforms[i].rotate_euler(0.0, 0.0, self.spins[i] * dt);
                i += 1;
            }
        }
    }

    /// render the particles to the current texture
    pub fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw_instances(&mut self.instances);
    }

    /// spawn a batch of particles
    fn spawn_particles(&mut self, count: usize) {
        if count == 0 { return; }

        let mut rng = rand::thread_rng();

        for _ in 0..count {
            let speed = self.dist.speed.sample(&mut rng).max(0.001);
            let lifespan = self.dist.lifespan.sample(&mut rng).max(0.001);
            let size = self.dist.size.sample(&mut rng).max(0.001);
            let spin = self.dist.spin.sample(&mut rng);

            let angle = random::<f32>() * 2.0 * PI;
            let direction = Vec3 { x: angle.cos(), y: angle.sin(), z: 0.0};
            let velocity = direction * speed * random::<f32>();

            let z_rotation = self.dist.rotation.sample(&mut rng);
            let init_rotation = Quat::from_euler(glam::EulerRot::YXZ, 0.0, 0.0, z_rotation);

            self.instances.transforms.push(Transform::new(
                self.config.emit_center, 
                init_rotation, 
                Vec3 { x: size, y: size, z: 1.0 }
            ));
            self.velocities.push(velocity);
            self.lifetimes.push(0.0); // all particles are 'just born'
            self.lifespans.push(lifespan);
            self.spins.push(spin);
        }
    }
}