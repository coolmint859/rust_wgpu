#![allow(dead_code)]
use std::{f32::consts::PI, ops::Range, sync::atomic::AtomicU32};

use glam::{Vec3, Vec4};
use rand_distr::{Distribution, Normal};

use crate::{game::animation::{AnimationController, FadeMode}, graphics::{data_utils::DataTable, entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, instance::{InstanceGroup, InstanceTemplate, TintAttribute, TransformAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::GameSystem, transform::Transform, vertex::VertexData}};

static PARTICLE_SYS_COUNTER: AtomicU32 = AtomicU32::new(0);

const TIMELINE: &str = "lifetime";
const KINEMATICS: &str = "kinematics";
const FADE: &str = "fade";

/// Data struct for the lifetimes of particle (age + lifespan)
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleTimeline {
    pub age: f32,
    pub lifespan: f32,
}

/// Data struct for the movement (kinematics) of particles
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleKinematics {
    pub velocity: Vec3,
    pub spin: f32,
}

/// Data struct for the creation of normal distributions
pub struct Variance {
    pub mean: f32,
    pub std_dev: f32,
}

/// Represents spawning and simulation behavior for particles in a particle system
pub trait ParticleBehavior {
    /// Initialize any data properties needed for this behavior to function
    /// 
    /// * 'particles' - the table of cpu-side particle properties
    fn init_properties(&self, particles: &mut DataTable);

    /// spawn new particles and their data
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'particles' - the table of cpu-side particle properties
    /// * 'spawn_range' - the range of new particles that need to their data to be reset/created
    fn spawn(&self, instances: &mut VertexData, particles: &mut DataTable, range: Range<usize>);

    /// Simulate the behavior for currently alive particles
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'particles' - the table of cpu-side particle properties
    /// * 'count' - the number of particles to simulate (usually all active ones)
    /// * 'dt' the delta time of the last frame
    fn simulate(&self, instances: &mut VertexData, particles: &DataTable, count: usize, dt: f32);
}

/// Configuration struct for the particle system
pub struct ParticleConfig {
    /// the total number of particles to render
    pub total_particles: usize,
    /// the number of particles to emit per update
    pub emit_cap: usize,
    /// the path to the texture to render on the particles
    pub texture_path: &'static str,
    /// if true, only one burst of particles will occur, otherwise 'dead' particles
    /// will be reborn with all properties randomized.
    pub is_one_shot: bool,
    /// the mean and standard deviation of particle lifetimes
    pub lifespans: Variance,
    /// the mean ans standard deviation of particle sizes
    pub sizes: Variance
}

pub struct ParticleEmitter2D {
    /// the particle instances that will be simulated
    particles: Entity,
    /// template for creating new particle instances
    particle_template: InstanceTemplate,
    /// the set of cpu-side particle data (lifetime, spin, velocity, etc)
    particle_data: DataTable,
    /// the set of behaviors that determine how particles are spawned and simulated
    behaviors: Vec<Box<dyn ParticleBehavior>>,
    /// configuration for the emitter
    config: ParticleConfig,
    /// the distribution sampler for particle lifetimes
    lifetime_dist: Normal<f32>,
    /// the distibution sampler for particle sizes,
    size_dist: Normal<f32>,

    animator: Option<Box<dyn AnimationController>>
}

impl ParticleEmitter2D {
    pub fn new(config: ParticleConfig) -> Self {
        let geometry = Geometry::new(Shape2D::new().square())
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

        let particle_data = DataTable::new(config.total_particles)
            .with_property::<ParticleTimeline>(TIMELINE);

        let particle_template = InstanceTemplate::new()
            .with_attribute(TransformAttribute, Transform::default())
            .with_attribute(TintAttribute, Vec4::ONE);

        let mut emitter = Self {
            particles,
            particle_template,
            particle_data,
            behaviors: Vec::new(),
            lifetime_dist: Normal::<f32>::new(config.lifespans.mean, config.lifespans.std_dev).unwrap(),
            size_dist: Normal::<f32>::new(config.sizes.mean, config.sizes.std_dev).unwrap(),
            config,
            animator: None,
        };
        emitter.spawn_particles(emitter.config.emit_cap);

        emitter
    }

    /// Add a behavior to to this particle system
    pub fn with_behavior(mut self, behavior: impl ParticleBehavior + 'static) -> Self {
        self.add_behavior(behavior);
        self
    }

    /// Add a behavior to to this particle system
    pub fn add_behavior(&mut self, behavior: impl ParticleBehavior + 'static) {
        behavior.init_properties(&mut self.particle_data);

        // need to add initial values to the active particles so they can updated properly
        let count = self.particles.instances.count();
        behavior.spawn(
            self.particles.instances.get_instances_mut(), 
            &mut self.particle_data, 
            0..count,
        );

        self.behaviors.push(Box::new(behavior));
    }

    /// Provide an animation controller for the particle emitter as a whole.
    /// 
    /// Note: Animators animate all entity instances uniformly. For per-instance animations,
    /// add a behavior to the emitter instead.
    pub fn set_animator<A: AnimationController + 'static>(&mut self, animator: A) {
        self.animator = Some(Box::new(animator));
    }

    /// spawn a batch of particles
    fn spawn_particles(&mut self, count: usize) {
        let rng = &mut rand::thread_rng();

        let curr_count = self.particles.instances.count();
        for _ in 0..count {
            let lifespan = self.lifetime_dist.sample(rng);

            let _ = self.particle_data.push_to(
                TIMELINE, 
                ParticleTimeline { lifespan, age: 0.0 }
            );

            let size = self.size_dist.sample(rng);
            self.particle_template.set_transform(
                Transform::default().with_scale(Vec3::new(size, size, 1.0))
            );

            self.particles.instances.add_instance(self.particle_template.clone());
        }
        
        for behavior in self.behaviors.iter() {
            let spawn_range = curr_count..self.particles.instances.count();
            
            behavior.spawn(
                self.particles.instances.get_instances_mut(), 
                &mut self.particle_data, 
                spawn_range,
            );
        }
    }
}

impl GameSystem for ParticleEmitter2D {
    fn update(&mut self, dt: f32, et: f32) {
        // spawn particles if not one shot
        if !self.config.is_one_shot {
            let available = self.config.total_particles - self.particles.instances.count();
            let to_spawn = available.min(self.config.emit_cap);
            
            if to_spawn > 0 {
                self.spawn_particles(to_spawn);
            }
        }

        // prune dead particles
        let mut idx = 0;
        if let Some(timelines) = self.particle_data.get_property_mut::<ParticleTimeline>(TIMELINE) {
            // need a raw pointer to modify particle data in-loop
            let timelines_ptr = timelines.as_mut_ptr();
        
            while idx < self.particles.instances.count() {
                unsafe {
                    let timeline = &mut *timelines_ptr.add(idx);
                    timeline.age += dt;

                    if timeline.age < timeline.lifespan {
                        idx += 1;
                        continue;
                    }
                }

                // we can remove the particle since the timeline Vec was dropped at the beginning of the loop
                self.particle_data.swap_remove_all(idx);
                self.particles.instances.get_instances_mut().swap_remove(idx);
            }
        }
        
        let active_count = self.particles.instances.count();
        if active_count == 0 { return; }

        // animate active particles
        if let Some(animator) = &mut self.animator {
            animator.animate(&mut self.particles, dt, et);
        }

        // simulate active particles
        for behavior in &self.behaviors {
            behavior.simulate(
                self.particles.instances.get_instances_mut(), 
                &self.particle_data,
                active_count,
                dt
            );
        }
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw(&mut self.particles);
    }
}

/// Particle behavior for radial movement (random velocity from a emission center point)
pub struct RadialKinematicsBehavior {
    pub speed_dist: Normal<f32>,
    pub spin_dist: Normal<f32>,
    pub emit_center: Vec3,
}

impl RadialKinematicsBehavior {
    pub fn new(speed: Variance, spin: Variance, emit_center: Vec3) -> Self {
        Self {
            speed_dist: Normal::<f32>::new(speed.mean, speed.std_dev).unwrap(),
            spin_dist: Normal::<f32>::new(spin.mean, spin.std_dev).unwrap(),
            emit_center,
        }
    }
}

impl ParticleBehavior for RadialKinematicsBehavior {
    fn init_properties(&self, particles: &mut DataTable) {
        particles.add_property::<ParticleKinematics>(KINEMATICS);
    }

    fn spawn(&self, instances: &mut VertexData, particles: &mut DataTable, spawn_range: Range<usize>) {
        let transforms_opt = instances.get_attribute_mut::<Transform>(TransformAttribute);
        let kinematics_opt = particles.get_property_mut::<ParticleKinematics>(KINEMATICS);
        
        if let (Some(transforms), Some(kinematics)) = (transforms_opt, kinematics_opt) {
            let rng = &mut rand::thread_rng();

            for i in spawn_range {
                transforms[i].move_to(self.emit_center);

                let speed = self.speed_dist.sample(rng);
                let angle = rand::random::<f32>() * 2.0 * PI; // random number between 0 & 2PI

                let velocity = Vec3::new(angle.cos() * speed, angle.sin() * speed, 0.0);
                let spin = self.spin_dist.sample(rng);

                kinematics.push(ParticleKinematics { velocity, spin });
            }
        }
    }

    fn simulate(&self, instances: &mut VertexData, particles: &DataTable, count: usize, dt: f32) {
        let transforms_opt = instances.get_attribute_mut::<Transform>(TransformAttribute);
        let kinematics_opt = particles.get_property::<ParticleKinematics>(KINEMATICS);

        if let (Some(transforms), Some(kinematics)) = (transforms_opt, kinematics_opt) {
            for i in 0..count {
                transforms[i].translate(kinematics[i].velocity * dt);
                transforms[i].rotate_euler(0.0, 0.0, kinematics[i].spin * dt);
            }
        }
    }
}


/// Causes particles to have a fade animation over their lifetimes
pub struct FadeBehavior {
    mode: FadeMode
}

impl FadeBehavior {
    pub fn new(mode: FadeMode) -> Self {
        Self { mode }
    }
}

impl ParticleBehavior for FadeBehavior {
    fn init_properties(&self, _particles: &mut DataTable) { }
    fn spawn(&self, _instances: &mut VertexData, _particles: &mut DataTable, _spawn_range: Range<usize>) {}

    fn simulate(&self, instances: &mut VertexData, particles: &DataTable, count: usize, _dt: f32) {
        let timelines_opt = particles.get_property::<ParticleTimeline>(TIMELINE);
        let tints_opt = instances.get_attribute_mut::<Vec4>(TintAttribute);
        
        if let (Some(timelines), Some(tints)) = (timelines_opt, tints_opt) {
            for i in 0..count {
                tints[i].w = self.mode.get_alpha(timelines[i].age, timelines[i].lifespan);
            }
        }
    }
}