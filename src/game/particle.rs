#![allow(dead_code)]
use std::{f32::consts::PI, ops::Range, sync::atomic::AtomicU32};

use glam::{Vec3, Vec4};
use rand_distr::{Distribution, Normal};

use crate::{game::animation::{AnimationController, FadeMode}, graphics::{data_table::{DataTable, DirtyVec}, entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, instance::{InstanceGroup, InstanceTemplate, TINT_ATTR, TRANSFORM_ATTR, TintAttribute, TransformAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, renderer::Renderer, shape_factory::Shape2D, traits::GameSystem, transform::Transform}};

static PARTICLE_SYS_COUNTER: AtomicU32 = AtomicU32::new(0);

const TIMELINE: &str = "lifetime";
const VELOCITY: &str = "velocity";
const SPIN: &str = "spin";
const WIND_DRAG: &str = "wind_drag";
const FADE: &str = "fade";

/// Data struct for the lifetimes of particle (age + lifespan)
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleTimeline {
    pub age: f32,
    pub lifespan: f32,
}

/// Data struct for the creation of normal distributions
pub struct Variance {
    pub mean: f32,
    pub std_dev: f32,
}

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
            .with_attribute(TINT_ATTR, Vec4::ONE); // override tint defualt to be all 1s

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

/// Spawns particles at a point in space, pruning based on lifetimes.
pub struct PointSpawner2D {
    pub size_dist: Normal<f32>,
    pub lifespan_dist: Normal<f32>,
    pub emit_center: Vec3,
}

impl PointSpawner2D {
    pub fn new(lifespan: Variance, size: Variance, emit_center: Vec3) -> Self {
        Self {
            lifespan_dist: Normal::<f32>::new(lifespan.mean, lifespan.std_dev).unwrap(),
            size_dist: Normal::<f32>::new(size.mean, size.std_dev).unwrap(),
            emit_center,
        }
    }
}

impl ParticleInit for PointSpawner2D {
    fn init_properties(&self, properties: &mut DataTable) {
        properties.add_property(TIMELINE, |cap| {
            Vec::<ParticleTimeline>::with_capacity(cap)
        });
    }
}

impl ParticleLifeCycle for PointSpawner2D {
    fn spawn(&self, instances: &mut InstanceGroup, particles: &mut DataTable, range: Range<usize>) {
        let instance_table = instances.get_instances_mut();
        let transforms_opt = instance_table.get_property_mut::<DirtyVec<Transform>>(TRANSFORM_ATTR);
        let timelines_opt = particles.get_property_mut::<Vec<ParticleTimeline>>(TIMELINE);
        
        if let (Some(transforms), Some(timelines)) = (transforms_opt, timelines_opt) {
            let rng = &mut rand::thread_rng();
            let transforms = transforms.as_vec_mut();

            for i in range {
                let size = self.size_dist.sample(rng);
                transforms[i].set_scale(Vec3::new(size, size, 1.0));
                transforms[i].move_to(self.emit_center);

                let lifespan = self.lifespan_dist.sample(rng);
                timelines.push(ParticleTimeline { age: 0.0, lifespan })
            }
        }
    }

    fn prune(&self, instances: &mut InstanceGroup, particles: &mut DataTable, count: usize, dt: f32) -> usize {
        let particle_proxy = particles.borrow_mut();
        if let Some(mut timelines) = particle_proxy.get_property_mut::<Vec<ParticleTimeline>>( TIMELINE) {
            let mut remaining = count;

            let mut idx = 0;
            while idx < remaining {
                timelines[idx].age += dt;

                if timelines[idx].age < timelines[idx].lifespan {
                    idx += 1;
                    continue;
                }

                particle_proxy.swap_remove_all(idx);
                instances.remove_instance(idx);
                remaining -=1;
            }
            
            return remaining;
        }
        return count;
    }
}

/// Simulates particles moving in a linear motion in a random direction on a plane
pub struct RadialKinematicsBehavior {
    pub speed_dist: Normal<f32>,
    pub spin_dist: Normal<f32>,
}

impl RadialKinematicsBehavior {
    pub fn new(speed: Variance, spin: Variance) -> Self {
        Self {
            speed_dist: Normal::<f32>::new(speed.mean, speed.std_dev).unwrap(),
            spin_dist: Normal::<f32>::new(spin.mean, spin.std_dev).unwrap(),
        }
    }
}

impl ParticleInit for RadialKinematicsBehavior {
    fn init_properties(&self, properties: &mut DataTable) {
        properties.add_property(VELOCITY, |cap| {
            Vec::<Vec3>::with_capacity(cap)
        });
        properties.add_property(SPIN, |cap| {
            Vec::<f32>::with_capacity(cap)
        });
    }
}

impl ParticleBehavior for RadialKinematicsBehavior {
    fn catch_up(&self, _instances: &mut InstanceGroup, properties: &mut DataTable, range: Range<usize>) {
        let prop_token = properties.borrow_mut();
        let velocities_opt = prop_token.get_property_mut::<Vec<Vec3>>(VELOCITY);
        let spins_opt = prop_token.get_property_mut::<Vec<f32>>(SPIN);

        if let (Some(mut velocities), Some(mut spins)) = (velocities_opt, spins_opt) {
            let rng = &mut rand::thread_rng();
            for _ in range {
                let speed = self.speed_dist.sample(rng);
                let angle = rand::random::<f32>() * 2.0 * PI; // random number between 0 & 2PI
                let velocity = Vec3::new(angle.cos(), angle.sin(), 0.0) * speed;
                
                let spin = self.spin_dist.sample(rng);

                velocities.push(velocity);
                spins.push(spin);
            }
        }
    }

    fn simulate(&self, instances: &mut InstanceGroup, properties: &mut DataTable, count: usize, dt: f32) {
        let velocities_opt = properties.get_property::<Vec<Vec3>>(VELOCITY);
        let spins_opt = properties.get_property::<Vec<f32>>(SPIN);

        let instance_table = instances.get_instances_mut();
        let transforms_opt = instance_table.get_property_mut::<DirtyVec<Transform>>(TRANSFORM_ATTR);

        if let (Some(transforms), Some(velocities), Some(spins)) = (transforms_opt, velocities_opt, spins_opt) {
            let transforms = transforms.as_vec_mut();
            
            for i in 0..count {
                transforms[i].translate(velocities[i] * dt);
                transforms[i].rotate_euler(0.0, 0.0, spins[i] * dt);
            }
        }
    }
}

/// Causes particles to have a fade animation over their lifetimes
pub struct FadeBehavior {
    mode: FadeMode
}

impl FadeBehavior {
    pub fn new(mode: FadeMode) -> Self { Self { mode } }
}

impl ParticleInit for FadeBehavior {
    fn init_properties(&self, properties: &mut DataTable) {
        properties.add_property(TIMELINE, |cap| {
            Vec::<ParticleTimeline>::with_capacity(cap)
        });
    }
}

impl ParticleBehavior for FadeBehavior {
    fn simulate(&self, instances: &mut InstanceGroup, particles: &mut DataTable, count: usize, _dt: f32) {
        let timelines_opt = particles.get_property::<Vec<ParticleTimeline>>(TIMELINE);

        let instance_table = instances.get_instances_mut();
        let tints_opt = instance_table.get_property_mut::<DirtyVec<Vec4>>(TINT_ATTR);
        
        if let (Some(timelines), Some(tints)) = (timelines_opt, tints_opt) {
            let tints = tints.as_vec_mut();
            
            for i in 0..count {
                tints[i].w = self.mode.get_alpha(timelines[i].age, timelines[i].lifespan);
            }
        }
    }
}
