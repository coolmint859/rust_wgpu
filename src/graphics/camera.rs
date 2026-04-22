#![allow(dead_code)]
use std::cell::Cell;
use glam::{Mat4, Quat, Vec2, Vec3};

use super::{
    bind_group::*, 
    transform::Transform
};

pub trait Camera {
    /// Get the unique identifier for this camera
    fn get_key(&self) -> String;

    /// Get the bind group layout builder for this camera
    fn get_layout_builder(&self) -> BindGroupLayoutBuilder;

    /// Get the view-projection matrix
    fn get_view_proj_mat(&self) -> glam::Mat4;

    /// get the position of the camera in 3D space
    fn get_position(&self) -> Vec3;

    /// Check if the camera data has changed
    fn is_dirty(&self) -> bool;

    /// set the aspect ratio for the camera's projection (should be called when the target dimensions change)
    fn set_aspect_ratio(&mut self, new_aspect: f32);

    /// Trigger the camera to update it's view-projection matrix
    fn update(&mut self);
}


/// Represents a 2D camera, using orthographic projection
pub struct Camera2D {
    key: String,
    transform: Transform,
    zoom: f32, 
    aspect: f32,
    is_dirty: Cell<bool>,

    view_proj_mat: Mat4,
}

impl Camera2D {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            transform: Transform::default(),
            zoom: 1.0,
            aspect: 1.0,
            is_dirty: Cell::new(true),
            view_proj_mat: glam::Mat4::IDENTITY
        }
    }

    /// Define a Camera2D with an initial position
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.transform.move_world(position);
        self
    }

    /// Define a Camera2D with an initial rotation
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.transform.set_rotation(rotation);
        self
    }

    /// Define a Camera2D with an initial zoom.
    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Move the camera relative to the local origin
    pub fn translate(&mut self, amount: Vec2) {
        self.transform.translate(Vec3::new(amount.x, amount.y, 0.0));
        self.is_dirty.set(true);
    }

    /// Move the camera relative the world origin
    pub fn move_to(&mut self, position: Vec2) {
        self.transform.move_world(Vec3::new(position.x, position.y, 0.0));
        self.is_dirty.set(true);
    }

    /// Tilt the camera to the left or right relative to local z-axis
    pub fn tilt_local(&mut self, tilt: f32) {
        let rotation = self.transform.rotation * glam::Quat::from_rotation_z(tilt);
        self.transform.set_rotation(rotation);
        self.is_dirty.set(true);
    }

    /// Tilt the camera to the left or right relative to world z-axis
    pub fn tilt_world(&mut self, tilt: f32) {
        let rotation = glam::Quat::from_rotation_z(tilt) * self.transform.rotation;
        self.transform.set_rotation(rotation);
        self.is_dirty.set(true);
    }

    /// Set the absolute tilt of the camera 
    pub fn set_tilt(&mut self, tilt: f32) {
        let rotation = glam::Quat::from_rotation_z(tilt);
        self.transform.set_rotation(rotation);
        self.is_dirty.set(true);
    }

    /// set the zoom of the camera
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
        self.is_dirty.set(true);
    }
}

impl Camera for Camera2D {
    fn get_key(&self) -> String {
        self.key.clone()
    }

    fn get_layout_builder(&self) -> BindGroupLayoutBuilder {
        BindGroupLayoutBuilder::new()
            .with_label("camera-2d")
            .with_entry(LayoutEntry {
                binding: 0,
                visibility: LayoutVisibility::VertexFragment,
                ty: LayoutBindType::Uniform,
            })
    }

    fn get_position(&self) -> Vec3 {
        self.transform.position
    }

    fn get_view_proj_mat(&self) -> glam::Mat4 {
        self.view_proj_mat.clone()
    }

    fn is_dirty(&self) -> bool {
        self.is_dirty.get() || self.transform.is_dirty()
    }

    fn set_aspect_ratio(&mut self, new_aspect: f32) {
        if self.aspect != new_aspect {
            self.aspect = new_aspect;
            self.is_dirty.set(true);
        }
    }

    /// update the camera's view-projection matrix
    fn update(&mut self) {
        if self.is_dirty() {
            self.transform.update();
            let view_mat = self.transform.world_matrix().inverse();

            let half_width = self.aspect / self.zoom;
            let half_height = 1.0 / self.zoom;

            let proj_mat = glam::Mat4::orthographic_lh(
                -half_width, 
                half_width, 
                -half_height, 
                half_height, 
                -1.0, 
                1.0
            );

            self.view_proj_mat = proj_mat * view_mat;
            self.is_dirty.set(false);
        }
    }
}