#![allow(dead_code)]
use std::{f32::consts::PI, ops::Range};

use glam::{Vec3, Vec4};
use rand_distr::{Distribution, Normal};

use crate::{game::{animation::FadeMode, particle::{ParticleBehavior, ParticleInit, ParticleLifeCycle}}, graphics::{data_table::DataTable, instance::{InstanceGroup, TintAttribute, TransformAttribute}, transform::Transform}};

const TIMELINE: &str = "lifetime";
const VELOCITY: &str = "velocity";
const SPIN: &str = "spin";
const WIND_DRAG: &str = "wind_drag";
const DROP_DELAY: &str = "drop_delay";
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
        properties.add_property(TIMELINE, |cap| Vec::<ParticleTimeline>::with_capacity(cap));
    }
}

impl ParticleLifeCycle for PointSpawner2D {
    fn spawn(&self, instances: &mut InstanceGroup, particles: &mut DataTable, range: Range<usize>) {
        let transforms_opt = instances.get_attribute_mut::<Transform>(TransformAttribute);
        let timelines_opt = particles.get_property_mut::<Vec<ParticleTimeline>>(TIMELINE);
        
        if let (Some(transforms), Some(timelines)) = (transforms_opt, timelines_opt) {
            let rng = &mut rand::thread_rng();

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
        properties.add_property(VELOCITY, |cap| Vec::<Vec3>::with_capacity(cap));
        properties.add_property(SPIN, |cap| Vec::<f32>::with_capacity(cap));
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

        let transforms_opt = instances.get_attribute_mut::<Transform>(TransformAttribute);

        if let (Some(transforms), Some(velocities), Some(spins)) = (transforms_opt, velocities_opt, spins_opt) {
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
        properties.add_property(TIMELINE, |cap| Vec::<ParticleTimeline>::with_capacity(cap));
    }
}

impl ParticleBehavior for FadeBehavior {
    fn simulate(&self, instances: &mut InstanceGroup, particles: &mut DataTable, count: usize, _dt: f32) {
        let timelines_opt = particles.get_property::<Vec<ParticleTimeline>>(TIMELINE);
        let tints_opt = instances.get_attribute_mut::<Vec4>(TintAttribute);
        
        if let (Some(timelines), Some(tints)) = (timelines_opt, tints_opt) {
            for i in 0..count {
                tints[i].w = self.mode.get_alpha(timelines[i].age, timelines[i].lifespan);
            }
        }
    }
}

/// Spawns particles on a plane and pruned them when they reach some other plane
pub struct PlaneSpawner {
    pub emit_width: f32,
    pub sky_y: f32,
    pub floor_y: f32,
    pub max_size: f32,
}

impl ParticleInit for PlaneSpawner {}

impl ParticleLifeCycle for PlaneSpawner {
    fn spawn(&self, instances: &mut InstanceGroup, _properties: &mut DataTable, range: Range<usize>) {
        if let Some(transforms) = instances.get_attribute_mut::<Transform>(TransformAttribute) {
            let rng = &mut rand::thread_rng();

            for i in range {
                let half_width = self.emit_width / 2.0;
                let x_pos = rand::Rng::gen_range(rng, -half_width..half_width);
                let z_pos = rand::Rng::gen_range(rng, 0.0..1.0);

                let size = lerp(self.max_size, 0.001, z_pos);

                transforms[i].move_to(Vec3::new(x_pos, self.sky_y, z_pos));
                transforms[i].set_scale(Vec3::new(size, size, 1.0));
            }
        }
    }

    fn prune(&self, instances: &mut InstanceGroup, properties: &mut DataTable, count: usize, _dt: f32) -> usize {
        let mut dead_instances = Vec::new();
        if let Some(transforms) = instances.get_attribute_mut::<Transform>(TransformAttribute) {
            for idx in 0..count {
                if transforms[idx].get_position().y <= self.floor_y {
                    dead_instances.push(idx);
                }
            }
        }

        if dead_instances.is_empty() { return count; }

        for &idx in dead_instances.iter().rev() {
            properties.swap_remove(idx);
            instances.remove_instance(idx);
        }

        return count - dead_instances.len();
    }
}

/// Data struct for particle movement delays
#[derive(Clone, Default, Debug)]
pub struct MovementDelay {
    pub wait_time: f32,
    pub move_time: f32,
}

/// linearly interpolate from start to end with the amount t
fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

/// Simulates particles falling according to weather
pub struct WeatherForceBehavior {
    pub gravity: f32,
    pub wind_force: f32,
    pub terminal_velocity: f32,
    pub max_delay: f32,
}

impl ParticleInit for WeatherForceBehavior {
    fn init_properties(&self, properties: &mut DataTable) {
        properties.add_property(VELOCITY, |cap| Vec::<Vec3>::with_capacity(cap));
        properties.add_property(DROP_DELAY, |cap| Vec::<MovementDelay>::with_capacity(cap));
    }
}

impl ParticleBehavior for WeatherForceBehavior {
    fn catch_up(&self, instances: &mut InstanceGroup, properties: &mut DataTable, range: Range<usize>) {
        let prop_proxy = properties.borrow_mut();
        let velocities_opt = prop_proxy.get_property_mut::<Vec<Vec3>>(VELOCITY);
        let delays_opt = prop_proxy.get_property_mut::<Vec<MovementDelay>>(DROP_DELAY);

        let transforms_opt = instances.get_attribute_mut::<Transform>(TransformAttribute);
        if let (Some(transforms), Some(mut velocities), Some(mut delays)) = (transforms_opt, velocities_opt, delays_opt) {
            let mut rng = rand::thread_rng();

            for i in range {
                velocities.push(Vec3::new(self.wind_force, self.gravity, 0.0));

                delays.push(MovementDelay { 
                    wait_time: 0.0, 
                    move_time: rand::Rng::gen_range(&mut rng, 0.0..self.max_delay)
                });

                let angle = velocities[i].y.atan2(velocities[i].x) - (PI / 2.0);
                transforms[i].rotate_euler(0.0, 0.0, angle);
            }
        }
    }

    fn simulate(&self, instances: &mut InstanceGroup, properties: &mut DataTable, count: usize, dt: f32) {
        let prop_proxy = properties.borrow_mut();
        let velocities_opt = prop_proxy.get_property_mut::<Vec<Vec3>>(VELOCITY);
        let delays_opt = prop_proxy.get_property_mut::<Vec<MovementDelay>>(DROP_DELAY);

        let transforms_opt = instances.get_attribute_mut::<Transform>(TransformAttribute);

        if let (Some(mut velocities), Some(mut delays), Some(transforms)) = (velocities_opt, delays_opt, transforms_opt) {
            for i in 0..count {
                delays[i].wait_time += dt;
                if delays[i].wait_time < delays[i].move_time { continue; }

                let parallax = lerp(1.0, 0.35, transforms[i].get_position().z);

                velocities[i].y += self.gravity * parallax * dt;
                velocities[i].y = velocities[i].y.max(-self.terminal_velocity * parallax);

                velocities[i].x = self.wind_force * parallax;

                transforms[i].translate(velocities[i] * dt);
            }
        }
    }
}