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

/// Represents simulation behavior for particles in a particle system
pub trait ParticleBehavior {
    /// Initialize any properties needed for this behavior to function.
    /// Defaults to doing nothing.
    /// 
    /// Is called when the behavior is first added to a ParticleEmitter
    /// 
    /// * 'particles' - the table of cpu-side particle properties
    fn init_properties(&self, _particles: &mut DataTable) {}

    /// Creates data for already active particles within the spawn range.
    /// Defaults to doing nothing.
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'particles' - the table of cpu-side particle properties
    /// * 'spawn_range' - the range of new particles that need to their data to be reset/created
    fn catch_up(&self, _instances: &mut DataTable, _particles: &mut DataTable, _range: Range<usize>) {}

    /// Simulate the behavior for currently active particles
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'particles' - the table of cpu-side particle properties
    /// * 'count' - the number of particles to simulate (usually all active ones)
    /// * 'dt' the delta time of the last frame
    fn simulate(&self, instances: &mut DataTable, particles: &mut DataTable, count: usize, dt: f32);
}

/// Represents spawning behavior for particles in a particle system
pub trait ParticleSpawner: ParticleBehavior {
    /// Spawn new particles within the specified range
    /// 
    /// * 'instances' - the table of particle instance attributes
    /// * 'particles' - the table of cpu-side particle properties
    /// * 'spawn_range' - the range of new particles that need to their data to be reset/created
    fn spawn(&self, instances: &mut DataTable, particles: &mut DataTable, range: Range<usize>);
}

/// Configuration struct for the particle system
pub struct ParticleConfig {
    /// the total number of particles to render
    pub total_particles: usize,
    /// the number of particles to emit per update
    pub emit_cap: usize,
    /// the spawner for particles
    pub spawner: Box<dyn ParticleSpawner + 'static>,
    /// the path to the texture to render on the particles
    pub texture_path: &'static str,
    /// if true, only one burst of particles will occur, otherwise 'dead' particles
    /// will be reborn with all properties randomized.
    pub is_one_shot: bool,
    /// the mean and standard deviation of particle lifetimes
    pub lifespans: Variance,
}

pub struct ParticleEmitter {
    /// the particle instances that will be simulated
    particles: Entity,
    /// the amount of particles that can be alive at any moment
    total_particles: usize,
    /// the cap of the number of particles that can be spawned per frame
    emit_cap: usize,
    /// template for creating new particle instances
    particle_template: InstanceTemplate,
    /// the set of cpu-side particle data (lifetime, spin, velocity, etc)
    particle_data: DataTable,

    /// the set of behaviors that determine how particles are simulated
    behaviors: Vec<Box<dyn ParticleBehavior>>,
    /// the spawner for particles
    spawner: Box<dyn ParticleSpawner>,
    /// the distribution sampler for particle lifetimes
    lifetime_dist: Normal<f32>,
    /// If true, particles will be created only once
    is_one_shot: bool,

    /// optional animation controller to animate all particles uniformly
    animator: Option<Box<dyn AnimationController>>
}

impl ParticleEmitter {
    pub fn new(
        particles: Entity, 
        total_particles: usize, 
        emit_cap: usize,
        spawner: Box<dyn ParticleSpawner + 'static>,
        lifespans: Variance,
        is_one_shot: bool
    ) -> Self {
        let mut particle_data = DataTable::new(total_particles)
            .with_property(TIMELINE, |cap| {
                Vec::<ParticleTimeline>::with_capacity(cap)
            });
        spawner.init_properties(&mut particle_data);

        let particle_template = InstanceTemplate::new()
            .with_attribute(TransformAttribute, Transform::default())
            .with_attribute(TintAttribute, Vec4::ONE);

        let mut emitter = Self {
            particles,
            total_particles,
            emit_cap,
            particle_template,
            particle_data,
            behaviors: Vec::new(),
            spawner,
            lifetime_dist: Normal::<f32>::new(lifespans.mean, lifespans.std_dev).unwrap(),
            animator: None,
            is_one_shot
        };
        emitter.spawn_particles(emit_cap);

        emitter
    }

    pub fn from_config(config: ParticleConfig) -> Self {
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

        ParticleEmitter::new(
            particles, 
            config.total_particles, 
            config.emit_cap, 
            config.spawner,
            config.lifespans,
            config.is_one_shot,
        )
    }

    /// Reset the particle emitter, clearing the data tables and spawning fresh particles.
    pub fn reset(&mut self) {
        self.particle_data.reset_properties();
        self.particles.instances.clear_instances();

        self.spawner.init_properties(&mut self.particle_data);
        self.spawner.spawn(
            self.particles.instances.get_instances_mut(), 
            &mut self.particle_data, 
            0..self.emit_cap,
        );
    }

    /// Set the spawner for this particle system.
    /// 
    /// Note: this calls reset() on the particle emitter.
    pub fn set_spawner(&mut self, spawner: impl ParticleSpawner + 'static) {
        self.spawner = Box::new(spawner);
        self.reset();
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
        behavior.catch_up(
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

            let _ = self.particle_data.push_to::<Vec<ParticleTimeline>>(
                TIMELINE, 
                ParticleTimeline { lifespan, age: 0.0 }
            );

            self.particles.instances.add_instance(self.particle_template.clone());
        }

        let spawn_range = curr_count..self.particles.instances.count();
        self.spawner.spawn(
            self.particles.instances.get_instances_mut(), 
            &mut self.particle_data, 
            spawn_range.clone(),
        );

        for behavior in self.behaviors.iter() {
            behavior.catch_up(
                self.particles.instances.get_instances_mut(), 
                &mut self.particle_data, 
                spawn_range.clone(),
            );
        }
    }
}

impl GameSystem for ParticleEmitter {
    fn update(&mut self, dt: f32, et: f32) {
        // spawn particles if not one shot
        if !self.is_one_shot {
            let available = self.total_particles - self.particles.instances.count();
            let to_spawn = available.min(self.emit_cap);
            
            if to_spawn > 0 {
                self.spawn_particles(to_spawn);
            }
        }

        let particle_proxy = self.particle_data.borrow_mut();
        if let Some(mut timelines) = particle_proxy.get_property_mut::<Vec<ParticleTimeline>>( TIMELINE) {
            let mut idx = 0;
            while idx < self.particles.instances.count() {
                timelines[idx].age += dt;

                if timelines[idx].age < timelines[idx].lifespan {
                    idx += 1;
                    continue;
                }

                particle_proxy.swap_remove_all(idx);
                self.particles.instances.remove_instance(idx);
            }
        }
        
        let active_count = self.particles.instances.count();
        if active_count == 0 { return; }

        // animate active particles
        if let Some(animator) = &mut self.animator {
            animator.animate(&mut self.particles, dt, et);
        }

        self.spawner.simulate(
            self.particles.instances.get_instances_mut(), 
            &mut self.particle_data,
            active_count,
            dt
        );

        // simulate active particles
        for behavior in &self.behaviors {
            behavior.simulate(
                self.particles.instances.get_instances_mut(), 
                &mut self.particle_data,
                active_count,
                dt
            );
        }
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw(&mut self.particles);
    }
}

/// Spawns particles from a center point in random radial directions on a 2D plane
pub struct RadialSpawner2D {
    pub speed_dist: Normal<f32>,
    pub spin_dist: Normal<f32>,
    pub size_dist: Normal<f32>,
    pub emit_center: Vec3,
}

impl RadialSpawner2D {
    pub fn new(speed: Variance, spin: Variance, size: Variance, emit_center: Vec3) -> Self {
        Self {
            speed_dist: Normal::<f32>::new(speed.mean, speed.std_dev).unwrap(),
            spin_dist: Normal::<f32>::new(spin.mean, spin.std_dev).unwrap(),
            size_dist: Normal::<f32>::new(size.mean, size.std_dev).unwrap(),
            emit_center,
        }
    }
}

impl ParticleBehavior for RadialSpawner2D {
    fn init_properties(&self, particles: &mut DataTable) {
        particles.add_property(VELOCITY, |cap| {
            Vec::<Vec3>::with_capacity(cap)
        });
        particles.add_property(SPIN, |cap| {
            Vec::<f32>::with_capacity(cap)
        });
    }

    fn simulate(&self, instances: &mut DataTable, particles: &mut DataTable, count: usize, dt: f32) {
        let velocities_opt = particles.get_property::<Vec<Vec3>>(VELOCITY);
        let spins_opt = particles.get_property::<Vec<f32>>(SPIN);
        let transforms_opt = instances.get_property_mut::<DirtyVec<Transform>>(TRANSFORM_ATTR);

        if let (Some(transforms), Some(velocities), Some(spins)) = (transforms_opt, velocities_opt, spins_opt) {
            let transforms = transforms.as_vec_mut();
            
            for i in 0..count {
                transforms[i].translate(velocities[i] * dt);
                transforms[i].rotate_euler(0.0, 0.0, spins[i] * dt);
            }
        }
    }
}

impl ParticleSpawner for RadialSpawner2D {
    fn spawn(&self, instances: &mut DataTable, particles: &mut DataTable, spawn_range: Range<usize>) {
        let particle_token = particles.borrow_mut();
        let velocities_opt = particle_token.get_property_mut::<Vec<Vec3>>(VELOCITY);
        let spins_opt = particle_token.get_property_mut::<Vec<f32>>(SPIN);
        
        let transforms_opt = instances.get_property_mut::<DirtyVec<Transform>>(TRANSFORM_ATTR);

        if let (Some(transforms), Some(mut velocities), Some(mut spins)) = (transforms_opt, velocities_opt, spins_opt) {
            let rng = &mut rand::thread_rng();
            let transforms = transforms.as_vec_mut();

            for i in spawn_range {
                let size = self.size_dist.sample(rng);
                transforms[i].set_scale(Vec3::new(size, size, 1.0));
                transforms[i].move_to(self.emit_center);

                let speed = self.speed_dist.sample(rng);
                let angle = rand::random::<f32>() * 2.0 * PI; // random number between 0 & 2PI

                let velocity = Vec3::new(angle.cos() * speed, angle.sin() * speed, 0.0);
                let spin = self.spin_dist.sample(rng);

                velocities.push(velocity);
                spins.push(spin);
            }
        }
    }
}

// /// Spawns particles along a horizontal line and causes them to fall down according to gravity
// pub struct RainSpawner2D {
//     pub emit_width: f32,
//     pub spawn_height: f32,
//     pub base_fall_speed: f32,
//     pub max_depth: f32,

//     pub physics: WeatherForceBehavior
// }

// impl ParticleBehavior for RainSpawner2D {
//     fn init_properties(&self, particles: &mut DataTable) {
//         particles.add_property(VELOCITY, |cap| {
//             Vec::<Vec3>::with_capacity(cap)
//         });
//     }

//     fn simulate(&self, instances: &mut DataTable, particles: &mut DataTable, count: usize, dt: f32) {
//         self.physics.simulate(instances, particles, count, dt);
//     }
// }

// impl ParticleSpawner for RainSpawner2D {
//     fn spawn(&self, instances: &mut DataTable, particles: &mut DataTable, range: Range<usize>) {
//         let transforms_opt = instances.get_property_mut::<DirtyVec<Transform>>(TRANSFORM_ATTR);
//         let velocities_opt = particles.get_property_mut::<Vec<Vec3>>(VELOCITY);

//         if let (Some(transforms), Some(velocities)) = (transforms_opt, velocities_opt) {
//             let transforms = transforms.as_vec_mut();

//             let mut rng = rand::thread_rng();

//             for i in range {
//                 let x_range = (-self.emit_width / 2.0)..(self.emit_width/2.0);
//                 let x_pos = rand::Rng::gen_range(&mut rng, x_range);
//                 let depth = rand::Rng::gen_range(&mut rng, 0.0..self.max_depth);
//                 let parallax = 1.0 / (1.0 + depth);

//                 transforms[i].move_to(Vec3::new(x_pos, self.spawn_height, -depth));
//                 transforms[i].set_scale(Vec3::new(parallax, parallax, 1.0));

//                 let fall = -self.base_fall_speed * parallax;
//                 velocities.push(Vec3::new(0.0, fall, 0.0));
//             }
//         }
//     }
// }

/// Causes particles to have a fade animation over their lifetimes
pub struct FadeBehavior {
    mode: FadeMode
}

impl FadeBehavior {
    pub fn new(mode: FadeMode) -> Self { Self { mode } }
}

impl ParticleBehavior for FadeBehavior {
    fn simulate(&self, instances: &mut DataTable, particles: &mut DataTable, count: usize, _dt: f32) {
        let timelines_opt = particles.get_property::<Vec<ParticleTimeline>>(TIMELINE);
        let tints_opt = instances.get_property_mut::<DirtyVec<Vec4>>(TINT_ATTR);
        
        if let (Some(timelines), Some(tints)) = (timelines_opt, tints_opt) {
            let tints = tints.as_vec_mut();
            
            for i in 0..count {
                tints[i].w = self.mode.get_alpha(timelines[i].age, timelines[i].lifespan);
            }
        }
    }
}

// pub struct WeatherForceBehavior {
//     pub gravity: f32,
//     pub wind_force: f32,
// }

// impl ParticleBehavior for WeatherForceBehavior {
//     fn simulate(&self, instances: &mut DataTable, particles: &mut DataTable, count: usize, dt: f32) {
//         let transforms_opt = instances.get_property_mut::<DirtyVec<Transform>>(TRANSFORM_ATTR);
//         let velocities_opt = particles.get_property_mut::<Vec<Vec3>>(VELOCITY);

//         if let (Some(transforms), Some(velocities)) = (transforms_opt, velocities_opt) {
//             let transforms = transforms.as_vec_mut();

//             for i in 0..count {
//                 velocities[i].x += self.wind_force * dt;
//                 velocities[i].y += self.gravity * dt;

//                 transforms[i].translate(velocities[i] * dt);

//                 let tilt = velocities[i].y.atan2(velocities[i].y) + (PI / 2.0);
//                 transforms[i].rotate_euler(0.0, 0.0, tilt);
//             }
//         }
//     }
// }