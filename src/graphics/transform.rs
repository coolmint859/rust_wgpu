#![allow(dead_code)]
use std::cell::Cell;

use glam::*;

// local axis vectors
pub const LOCAL_RIGHT:Vec3 =    Vec3::new(1.0, 0.0, 0.0);
pub const LOCAL_UP:Vec3 =       Vec3::new(0.0, 1.0, 0.0);
pub const LOCAL_FORWARD:Vec3 =  Vec3::new(0.0, 0.0, 1.0);

/// represents position, rotation, and scaling of an entity
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub dimensions: Vec3,

    world_mat:Mat4,
    is_dirty: Cell<bool>,
}

impl Transform {
    pub fn new(position:Vec3, rotation:Quat, dimensions:Vec3) -> Self {
        let world_mat = Mat4::from_scale_rotation_translation(dimensions, rotation, position);
        let is_dirty = Cell::new(true);
        Self { position, rotation, dimensions, world_mat, is_dirty }
    }

    /// Create a transform that 'faces' the z-axis at the origin with scale 1
    pub fn default() -> Self {
        let position = Vec3::ZERO;
        let rotation = Quat::IDENTITY;
        let dimensions = Vec3::ONE;

        let world_mat = Mat4::from_scale_rotation_translation(dimensions, rotation, position);
        let is_dirty = Cell::new(true);
        Self { position, rotation, dimensions, world_mat, is_dirty }
    }

    /// get a copy of this transform's world matrix
    pub fn world_matrix(&self) -> glam::Mat4 {
        self.is_dirty.set(false);
        self.world_mat.clone()
    }

    /// move relative to local origin
    pub fn translate(&mut self, amount: Vec3) {
        self.position += amount;
        self.is_dirty.set(true);
    }

    /// move relative to world origin
    pub fn move_world(&mut self, amount: Vec3) {
        self.position += self.rotation * amount;
        self.is_dirty.set(true);
    }

    /// rotate from current orientation
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
        self.is_dirty.set(true);
    }

    /// rotate from current orientation, using Euler angles
    pub fn rotate_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.rotation *= Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        self.is_dirty.set(true);
    }

    /// Set the absolute rotation of the transform
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.rotation = rotation;
        self.is_dirty.set(true);
    }

    /// Set the absolute rotation of the transform using Euler angles
    pub fn set_rotation_euler(&mut self, pitch: f32, yaw: f32, roll: f32) {
        self.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
        self.is_dirty.set(true);
    }

    /// reorient this transform to 'point' to a target
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let look_dir = self.position - target;
        self.rotation = Quat::from_mat4(&Mat4::look_at_rh(self.position, look_dir, up.normalize()));
        self.is_dirty.set(true);
    }

    /// apply this transform to a vector
    pub fn apply_to(&self, vector:Vec3) -> Vec3 {
        let vec4 = Vec4::new(vector.x, vector.y, vector.z, 1.0);
        let transformed = self.world_mat.mul_vec4(vec4);
        transformed.xyz()
    }

    /// check if this transform is dirty
    pub fn is_dirty(&self) -> bool {
        return self.is_dirty.get()
    }

    /// Update the world matrix from the currently set position, rotation, and scale
    pub fn update_world_mat(&mut self) {
        if self.is_dirty() {
            self.world_mat = Mat4::from_scale_rotation_translation(self.dimensions, self.rotation, self.position);
            self.is_dirty.set(false);
        }
    }
}