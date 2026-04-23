use std::f32::consts::PI;

use glam::{Quat, Vec3};
use rand::{Rng, random};
use rand_distr::{Distribution, Normal};

use crate::graphics::{
    entity::EntityInstances, 
    mesh::Mesh, 
    presets::{MaterialPreset, RenderPipeline}, 
    renderer::Renderer, 
    shape_factory::Shape2D, 
    transform::Transform
};

pub struct Variance {
    pub mean: f32,
    pub std_dev: f32,
}

pub struct ParticleConfig {
    /// the number of particles to render
    pub num_particles: usize,
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
    /// the texture to render on the particles
    pub texture_path: &'static str,
    /// if true, only one burst of particles will occur, otherwise 'dead' particles
    /// will be reborn with all properties randomized.
    pub is_one_shot: bool,
}

pub struct ParticleSystem {
    config: ParticleConfig,
    remaining: usize,
    instances: EntityInstances,
    velocities: Vec<Vec3>,
    spins: Vec<f32>,
    lifetimes: Vec<f32>,
    lifespans: Vec<f32>,
}

impl ParticleSystem {
    pub fn new(config: ParticleConfig) -> Self {
        let mut transforms = Vec::new();
        let mut velocities = Vec::new();
        let mut lifetimes = Vec::new();
        let mut lifespans = Vec::new();
        let mut spins = Vec::new();
        let remaining = config.num_particles;

        let mut rng = rand::thread_rng();
        let speed_dist = Normal::new(config.speed.mean, config.speed.std_dev).unwrap();
        let life_dist =  Normal::new(config.lifespan.mean, config.lifespan.std_dev).unwrap();
        let size_dist =  Normal::new(config.size.mean, config.size.std_dev).unwrap();
        let spin_dist = Normal::new(config.spin.mean, config.spin.std_dev).unwrap();
        let rot_dist = Normal::new(config.rotation.mean, config.rotation.std_dev).unwrap();

        for _ in 0..config.num_particles {
            let speed = speed_dist.sample(&mut rng).max(0.0001);
            let lifespan = life_dist.sample(&mut rng).max(0.0001);
            let size = size_dist.sample(&mut rng).max(0.0001);
            let spin = spin_dist.sample(&mut rng);

            // give the particle a random direction
            let angle = random::<f32>() * 2.0 * PI;
            let vel_x = angle.cos() * speed;
            let vel_y = angle.sin() * speed;
            let velocity = Vec3 { x: vel_x, y: vel_y, z: 0.0} * random::<f32>();

            let z_rotation = rot_dist.sample(&mut rng);
            let init_rotation = Quat::from_euler(glam::EulerRot::YXZ, 0.0, 0.0, z_rotation);

            transforms.push(Transform::new(config.emit_center, init_rotation, Vec3 { x: size, y: size, z: 1.0 }));
            velocities.push(velocity);
            lifetimes.push(0.0); // all particles are 'just born'
            lifespans.push(lifespan);
            spins.push(spin);
        }

        let pipeline = RenderPipeline::TexturedSpriteInstanced.get();
        let instances = EntityInstances {
            mesh: Mesh::new("particle", Shape2D::new().square(pipeline.primary_vertex_layouts())),
            material: MaterialPreset::TexturedSprite(config.texture_path).with_label("particle"),
            pipeline,
            transforms,
        };

        Self {
            config,
            remaining,
            instances,
            velocities,
            lifetimes,
            lifespans,
            spins,
        }
    }

    /// Update the particles in this particle system.
    pub fn update(&mut self, dt: f32) {
        if self.remaining == 0 { return; }

        let mut rng = rand::thread_rng();
        let speed_dist = Normal::new(self.config.speed.mean, self.config.speed.std_dev).unwrap();
        let life_dist =  Normal::new(self.config.lifespan.mean, self.config.lifespan.std_dev).unwrap();
        let size_dist =  Normal::new(self.config.size.mean, self.config.size.std_dev).unwrap();
        let spin_dist = Normal::new(self.config.spin.mean, self.config.spin.std_dev).unwrap();
        let rot_dist = Normal::new(self.config.rotation.mean, self.config.rotation.std_dev).unwrap();

        let mut i = 0;
        while i < self.lifetimes.len() {
            self.lifetimes[i] += dt;

            if self.lifetimes[i] >= self.lifespans[i] {
                if self.config.is_one_shot {
                    self.lifetimes.swap_remove(i);
                    self.lifespans.swap_remove(i);
                    self.velocities.swap_remove(i);
                    self.spins.swap_remove(i);
                    self.instances.transforms.swap_remove(i);

                    continue; 
                } else {
                    self.reset_particle(i, &mut rng, &size_dist, &life_dist, &speed_dist, &spin_dist, &rot_dist)
                }
            }

            self.instances.transforms[i].translate(self.velocities[i] * dt);
            self.instances.transforms[i].rotate_euler(0.0, 0.0, self.spins[i] * dt);
            i += 1;
        }
        
        self.remaining = self.lifetimes.len();
    }

    /// resets a particle to the current emit center with a 
    fn reset_particle(
        &mut self,
        i: usize,
        rng: &mut impl Rng, 
        size_dist: &Normal<f32>, 
        life_dist: &Normal<f32>, 
        speed_dist: &Normal<f32>,
        spin_dist: &Normal<f32>, 
        rot_dist: &Normal<f32>, 
    ) {
        self.lifetimes[i] = 0.0;
        self.lifespans[i] = life_dist.sample(rng).max(0.001);

        let speed = speed_dist.sample(rng).max(0.001);        
        let size = size_dist.sample(rng).max(0.001);

        let angle = random::<f32>() * 2.0 * PI;
        let vel_x = angle.cos() * speed;
        let vel_y = angle.sin() * speed;
        let velocity = Vec3 { x: vel_x, y: vel_y, z: 0.0} * random::<f32>();
        self.velocities[i] = velocity;

        let z_rotation = rot_dist.sample(rng);
        let init_rotation = Quat::from_euler(glam::EulerRot::YXZ, 0.0, 0.0, z_rotation);
        self.spins[i] = spin_dist.sample(rng);

        // Move back to the current emitter center
        self.instances.transforms[i].move_to(self.config.emit_center);
        self.instances.transforms[i].scale(Vec3 { x: size, y: size, z: 1.0 });
        self.instances.transforms[i].set_rotation(init_rotation);
    }

    /// Get the number of remaining particles left - this only changes if is_one_shot is true
    pub fn remaining_particles(&self) -> usize {
        self.remaining
    }

    /// Set the emit center for this particle system. Only reset particles will use this if is_one_shot is false.
    pub fn set_emit_center(&mut self, center: Vec3) {
        self.config.emit_center = center;
    }

    /// render the particles to the current texture
    pub fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw_instances(&mut self.instances);
    }
}