#![allow(dead_code)]
use std::{cell::Cell, sync::atomic::{AtomicU32, Ordering}};

use glam::*;

// local axis vectors
pub const LOCAL_RIGHT:Vec3 =    Vec3::new(1.0, 0.0, 0.0);
pub const LOCAL_UP:Vec3 =       Vec3::new(0.0, 1.0, 0.0);
pub const LOCAL_FORWARD:Vec3 =  Vec3::new(0.0, 0.0, 1.0);

static TRANSFORM_COUNTER: AtomicU32 = AtomicU32::new(0);

/// represents position, rotation, and scaling of an entity
#[derive(Clone, Debug)]
pub struct Transform {
    id: u32,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,

    world_mat:Mat4,
    is_dirty: Cell<bool>,
}

impl Transform {
    pub fn new(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        let id = TRANSFORM_COUNTER.fetch_add(1, Ordering::SeqCst);
        let world_mat = Mat4::from_scale_rotation_translation(scale, rotation, position);
        let is_dirty = Cell::new(true);
        Self { id, position, rotation, scale, world_mat, is_dirty }
    }

    /// Create a transform that 'faces' the z-axis at the origin with scale 1
    pub fn default() -> Self {
        let id = TRANSFORM_COUNTER.fetch_add(1, Ordering::SeqCst);
        let position = Vec3::ZERO;
        let rotation = Quat::IDENTITY;
        let scale = Vec3::ONE;

        let world_mat = Mat4::from_scale_rotation_translation(scale, rotation, position);
        let is_dirty = Cell::new(true);
        Self { id, position, rotation, scale, world_mat, is_dirty }
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

    pub fn id(&self) -> u32 {
        self.id.clone()
    }

    /// Get a copy of this transform's world matrix
    pub fn world_matrix(&self) -> glam::Mat4 {
        self.is_dirty.set(false);
        self.world_mat.clone()
    }

    /// Get the position of this transform
    pub fn get_position(&self) -> Vec3 {
        self.position.clone()
    }

    /// Get the rotation of this transform
    pub fn get_rotation(&self) -> Quat {
        self.rotation.clone()
    }

    /// Get the scale of this transform
    pub fn get_scale(&self) -> Vec3 {
        self.scale.clone()
    }

    /// Move relative to local origin
    pub fn translate(&mut self, amount: Vec3) {
        self.position += amount;
        self.is_dirty.set(true);
    }

    /// Move relative to world origin
    pub fn move_to(&mut self, position: Vec3) {
        self.position = position;
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
    pub fn set_scale(&mut self, scale: glam::Vec3) {
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
    pub fn to_updated(&mut self) -> bool {
        if self.is_dirty() {
            self.world_mat = Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);
            self.is_dirty.set(false);
            return true;
        }
        return false;
    }
}