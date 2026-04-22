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
    pub scale: Vec3,

    world_mat:Mat4,
    is_dirty: Cell<bool>,
}

impl Transform {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let world_mat = Mat4::from_scale_rotation_translation(scale, rotation, position);
        let is_dirty = Cell::new(true);
        Self { position, rotation, scale, world_mat, is_dirty }
    }

    /// Create a transform that 'faces' the z-axis at the origin with scale 1
    pub fn default() -> Self {
        let position = Vec3::ZERO;
        let rotation = Quat::IDENTITY;
        let scale = Vec3::ONE;

        let world_mat = Mat4::from_scale_rotation_translation(scale, rotation, position);
        let is_dirty = Cell::new(true);
        Self { position, rotation, scale, world_mat, is_dirty }
    }

    /// Set the postition of the transform relative to the world axis
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Set the scale of the transform
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Set the rotation of the transform relative to the local center
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Get a copy of this transform's world matrix
    pub fn world_matrix(&self) -> glam::Mat4 {
        self.is_dirty.set(false);
        self.world_mat.clone()
    }

    /// Move relative to local origin
    pub fn translate(&mut self, amount: Vec3) {
        self.position += amount;
        self.is_dirty.set(true);
    }

    /// Move relative to world origin
    pub fn move_world(&mut self, amount: Vec3) {
        self.position += self.rotation * amount;
        self.is_dirty.set(true);
    }

    /// Set the x value for this transform relative to the world origin
    pub fn set_x(&mut self, x: f32) {
        self.position.x = x;
        self.is_dirty.set(true);
    }

    /// Set the y value for this transform relative to the world origin
    pub fn set_y(&mut self, y: f32) {
        self.position.y = y;
        self.is_dirty.set(true);
    }

    /// Set the z value for this transform relative to the world origin
    pub fn set_z(&mut self, z: f32) {
        self.position.z = z;
        self.is_dirty.set(true);
    }

    /// Rotate from current orientation
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation *= rotation;
        self.is_dirty.set(true);
    }

    /// Rotate from current orientation, using Euler angles
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

    /// Reorient this transform to 'point' to a target
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        let look_dir = self.position - target;
        self.rotation = Quat::from_mat4(&Mat4::look_at_rh(self.position, look_dir, up.normalize()));
        self.is_dirty.set(true);
    }

    /// Set the scale of this transform
    pub fn scale(&mut self, scale: glam::Vec3) {
        self.scale = scale;
        self.is_dirty.set(true);
    }

    /// Apply this transform to a vector
    pub fn apply_to(&self, vector:Vec3) -> Vec3 {
        let vec4 = Vec4::new(vector.x, vector.y, vector.z, 1.0);
        let transformed = self.world_mat.mul_vec4(vec4);
        transformed.xyz()
    }

    /// Check if this transform is dirty
    pub fn is_dirty(&self) -> bool {
        return self.is_dirty.get()
    }

    /// Update the world matrix from the currently set position, rotation, and scale.
    /// 
    /// Returns true if the transform had changed this frame, false otherwise
    pub fn update(&mut self) -> bool {
        if self.is_dirty() {
            self.world_mat = Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);
            self.is_dirty.set(false);
            return true;
        }
        return false;
    }
}