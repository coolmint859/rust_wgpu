#![allow(dead_code)]
use std::{ops::Range, sync::atomic::AtomicU32};

use glam::Vec4;

use crate::{game::animation::{AnimationController}, graphics::{data_table::DataTable, entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, instance::{InstanceGroup, InstanceTemplate, TINT_ATTR, TintAttribute, TransformAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::GameSystem, transform::Transform}};

static PARTICLE_SYS_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Core trait for particle systems.
/// 
/// Ensures particles are initialized with the data they need for spawning, pruning, and simulation.
pub trait ParticleInit {
    /// Initialize any properties needed for the particles in an emitter.
    /// Defaults to doing nothing.
    /// 
    /// * 'properties' - the table of particle properties.
    fn init_properties(&self, _properties: &mut DataTable) {}
}

/// Controls simulation behavior for active particles in a particle system
pub trait ParticleBehavior: ParticleInit {
    /// Populates data for already active particles within the spawn range.
    /// Defaults to doing nothing.
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'properties' - the table of particle properties
    /// * 'spawn_range' - the range of particles that need to be 'caught up'
    fn catch_up(&self, _instances: &mut InstanceGroup, _properties: &mut DataTable, _range: Range<usize>) {}

    /// Simulate the behavior for currently active particles
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'properties' - the table of cpu-side particle properties
    /// * 'count' - the number of currently active particles - active particles are guarenteed to be at the beginning of the data table rows
    /// * 'dt' the delta time of the last frame
    fn simulate(&self, instances: &mut InstanceGroup, properties: &mut DataTable, count: usize, dt: f32);
}

pub trait ParticleLifeCycle: ParticleInit {
    /// Spawn new particles within the specified range
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'properties' - the table of cpu-side particle properties
    /// * 'range' - the range of new particles in the tables to be spawned
    fn spawn(&self, instances: &mut InstanceGroup, properties: &mut DataTable, range: Range<usize>);

    /// Prune existing particles from the emitter. Returns the number of active particles after pruning.
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'properties' - the table of cpu-side particle properties
    /// * 'count' - the number of currently active particles - active particles are guarenteed to be at the beginning of the data table rows
    fn prune(&self, instances: &mut InstanceGroup, properties: &mut DataTable, count: usize, dt: f32) -> usize;
}

/// Configuration struct for a particle emitter
pub struct ParticleConfig {
    /// the total number of particles to render
    pub total_particles: usize,
    /// the number of particles to emit per update
    pub emit_cap: usize,
    /// if true, only one burst of particles will occur, otherwise 'dead' particles
    /// will be reborn with all properties randomized.
    pub is_one_shot: bool,
}

pub struct ParticleEmitter<L: ParticleLifeCycle + 'static> {
    /// the configuration of the emitter
    config: ParticleConfig,
    /// the particle instances to be simulated
    particles: Entity,
    /// template for creating new particle instances
    particle_template: InstanceTemplate,
    /// the set of particle properties (e.g., lifetime, spin, velocity, etc)
    particle_props: DataTable,

    /// The lifecycle for particle spawning/pruning
    lifecycle: L,
    /// Behaviors for particle simulation
    behaviors: Vec<Box<dyn ParticleBehavior>>,

    /// optional animation controller to animate all particles uniformly
    animator: Option<Box<dyn AnimationController>>
}

impl<L: ParticleLifeCycle + 'static> ParticleEmitter<L> {
    pub fn new(config: ParticleConfig, particles: Entity, lifecycle: L) -> Self {
        let mut particle_props = DataTable::new(config.total_particles);
        lifecycle.init_properties(&mut particle_props);

        let particle_template = particles.get_template()
            .with_defaults()
            .with_attribute(TINT_ATTR, Vec4::ONE); // override tint default to be all 1s

        Self {
            config,
            particles,
            particle_template,
            particle_props,
            behaviors: Vec::new(),
            lifecycle,
            animator: None,
        }
    }

    /// Create a new particle emitter for a colored square
    pub fn colored(color: Vec4, config: ParticleConfig, lifecycle: L) -> Self {
        let geometry = Geometry::new(Shape2D::new().square())
            .with_attribute(PositionAttribute);

        let instance_group = InstanceGroup::new(0, config.total_particles)
            .with_label("particles")
            .with_attribute(TransformAttribute, Vec::<Transform>::with_capacity(config.total_particles));

        let particles = Entity::from_group(
            "particles",
            geometry,
            MaterialPreset::ColoredSprite(color.to_array()).with_label("particles"),
            instance_group,
            RenderInfo { 
                shader_path: ShaderSpecPreset::ColoredSprite.path(), 
                pipeline: RenderPipeline::ColoredSprite.get() 
            }
        );

        ParticleEmitter::new(config, particles, lifecycle)
    }

    /// Create a new particle emitter for a textured square
    pub fn textured(path: &str, config: ParticleConfig, lifecycle: L) -> Self {
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
            MaterialPreset::TexturedSprite(path.to_string()).with_label("particles"),
            instance_group,
            RenderInfo { 
                shader_path: ShaderSpecPreset::TexturedSprite.path(), 
                pipeline: RenderPipeline::TexturedSprite.get() 
            }
        );

        ParticleEmitter::new(config, particles, lifecycle)
    }

    /// Reset the particle emitter, clearing the data tables and spawning fresh particles.
    pub fn reset(&mut self) {
        self.particle_props.reset_properties();
        self.particles.instances.clear_instances();

        self.lifecycle.spawn(
            &mut self.particles.instances, 
            &mut self.particle_props, 
            0..self.config.emit_cap,
        );
    }

    /// Add a behavior to to this particle system
    pub fn with_behavior(mut self, behavior: impl ParticleBehavior + 'static) -> Self {
        self.add_behavior(behavior);
        self
    }

    /// Add a behavior to to this particle system
    pub fn add_behavior(&mut self, behavior: impl ParticleBehavior + 'static) {
        behavior.init_properties(&mut self.particle_props);

        // need to add initial values to the active particles so they can updated properly
        let count = self.particles.instances.count();
        behavior.catch_up(
            &mut self.particles.instances, 
            &mut self.particle_props, 
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
        let curr_count = self.particles.instances.count();
        for _ in 0..count {
            self.particles.instances.add_instance(self.particle_template.clone());
        }

        let spawn_range = curr_count..self.particles.instances.count();
        self.lifecycle.spawn(
            &mut self.particles.instances, 
            &mut self.particle_props, 
            spawn_range.clone(),
        );

        for behavior in self.behaviors.iter() {
            behavior.catch_up(
                &mut self.particles.instances, 
                &mut self.particle_props, 
                spawn_range.clone(),
            );
        }
    }
}

impl<L: ParticleLifeCycle + 'static> GameSystem for ParticleEmitter<L> {
    fn update(&mut self, dt: f32, et: f32) {
        // spawn particles if not one shot
        if !self.config.is_one_shot {
            let available = self.config.total_particles - self.particles.instances.count();
            let to_spawn = available.min(self.config.emit_cap);
            
            if to_spawn > 0 {
                self.spawn_particles(to_spawn);
            }
        }

        let curr_active = self.particles.instances.count();
        let remaining = self.lifecycle.prune(
            &mut self.particles.instances, 
            &mut self.particle_props,
            curr_active,
            dt,
        );
        if remaining == 0 { return; }

        // animate active particles
        if let Some(animator) = &mut self.animator {
            animator.animate(&mut self.particles, dt, et);
        }

        // simulate active particles
        for behavior in &self.behaviors {
            behavior.simulate(
                &mut self.particles.instances, 
                &mut self.particle_props,
                remaining,
                dt
            );
        }
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw(&mut self.particles);
    }
}
