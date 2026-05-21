#![allow(dead_code)]
use std::{f32::consts::PI, sync::atomic::{AtomicU32, Ordering}};

use glam::{Quat, Vec3, Vec4};
use rand::random;
use rand_distr::{Distribution, Normal};

use crate::graphics::{entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, instance::{InstanceGroup, InstanceTemplate, TintAttribute, TransformAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::AnimationController, transform::Transform};

static PARTICLE_SYS_COUNTER: AtomicU32 = AtomicU32::new(0);

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

/// Contains the states of a particle system
struct ParticleStates {
    pub velocities: Vec<Vec3>,
    pub spins: Vec<f32>,
    pub lifetimes: Vec<f32>,
    pub lifespans: Vec<f32>,
}

impl ParticleStates {
    pub fn new() -> Self {
        Self {
            velocities: Vec::new(),
            spins: Vec::new(),
            lifespans: Vec::new(),
            lifetimes: Vec::new(),
        }
    }
}

/// A 2D particle emitter using instanced rendering.
/// 
/// Emits particles uniformly in all directions from a center point.
/// 
/// Particle behavior is determined by normal distributions
pub struct ParticleEmitter2D {
    id: u32,
    /// the configuration of the particle system
    config: ParticleConfig,
    /// the distributions for particle behavior
    dist: ParticleDistributions,
    /// the particle instances for rendering
    particles: Entity,
    /// instance template for creating individual particles
    template: InstanceTemplate,
    /// particle velocities, spins, lifetimes, etc...
    states: ParticleStates
}

impl ParticleEmitter2D {
    pub fn new(config: ParticleConfig) -> Self {
        let id = PARTICLE_SYS_COUNTER.fetch_add(1, Ordering::SeqCst);
        
        let geometry_data = Shape2D::new().square();
        let geometry = Geometry::new(geometry_data.clone())
            .with_attribute(PositionAttribute)
            .with_attribute(UVAttribute);

        let instance_group = InstanceGroup::new(0, config.total_particles)
            .with_label("particles")
            .with_attribute(TransformAttribute, Vec::<Transform>::with_capacity(config.total_particles))
            .with_attribute(TintAttribute, Vec::<Vec4>::with_capacity(config.total_particles));

        let particles = Entity::from_group(
            "particles",
            geometry,
            MaterialPreset::TexturedSprite(config.texture_path.to_string()).with_label("particles"),
            instance_group,
            RenderInfo { 
                shader_path: ShaderSpecPreset::TexturedSprite.path(), 
                pipeline: RenderPipeline::TexturedSprite.get() 
            }
        );

        let template = InstanceTemplate::new()
            .with_transform(Transform::default())
            .with_attribute(TintAttribute, Vec4::ONE);

        let start_particles = config.spawn_cap;
        let dist = ParticleDistributions::new(&config);

        let mut system = Self {
            id,
            config,
            dist,
            particles,
            template,
            states: ParticleStates::new(),
        };

        system.spawn_particles(start_particles);

        system
    }

    /// spawn as many particles as possible
    pub fn burst(&mut self) {
        let count = self.config.total_particles - self.particles.instances.count();
        self.spawn_particles(count);
    }

    /// Get the number of remaining particles left - this only changes if is_one_shot is true
    pub fn remaining_particles(&self) -> usize {
        self.particles.instances.count()
    }

    /// Set the emit center for this particle system. Only reset particles will use this if is_one_shot is false.
    pub fn set_emit_center(&mut self, center: Vec3) {
        self.config.emit_center = center;
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

            let transform = Transform::new(
                self.config.emit_center,
                init_rotation,
                Vec3 { x: size, y: size, z: 0.0}
            );

            self.template.set_transform(transform);
            self.particles.instances.add_instance(self.template.clone());

            self.states.velocities.push(velocity);
            self.states.lifetimes.push(0.0); // all particles are 'just born'
            self.states.lifespans.push(lifespan);
            self.states.spins.push(spin);
        }
    }
}

impl AnimationController for ParticleEmitter2D {
    /// Update the particles in this particle system.
    fn update(&mut self, dt: f32) {
        // spawn new particles if continuous
        if !self.config.is_one_shot {
            let current_alive = self.particles.instances.count();
            let available_space = self.config.total_particles.saturating_sub(current_alive);
            let to_spawn = self.config.spawn_cap.min(available_space);

            self.spawn_particles(to_spawn);
        }

        // update all particles
        let mut i = 0;
        while i < self.particles.instances.count() {
            self.states.lifetimes[i] += dt;

            if self.states.lifetimes[i] >= self.states.lifespans[i] {
                self.states.lifetimes.swap_remove(i);
                self.states.lifespans.swap_remove(i);
                self.states.velocities.swap_remove(i);
                self.states.spins.swap_remove(i);
                
                self.particles.instances.remove_instance(i);
            } else {
                if let Some(mut instance) = self.particles.instances.get_instance_mut(i) {
                    let transform = instance.get_transform_mut().unwrap();
                    transform.translate(self.states.velocities[i] * dt);
                    transform.rotate_euler(0.0, 0.0, self.states.spins[i] * dt);

                    let tint = instance.get_attribute_mut::<Vec4>(TintAttribute).unwrap();
                    let life_ratio = self.states.lifetimes[i] / self.states.lifespans[i];
                    tint.w = (1.0 - life_ratio).clamp(0.0, 1.0);
                }
                i += 1;
            }
        }
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw(&mut self.particles);
    }
}