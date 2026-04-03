#![allow(dead_code)]
use std::cell::Cell;
use glam::{Mat4, Quat, Vec2, Vec3};

use crate::graphics::{material::UniformEntry, transform::Transform};

pub trait Camera {
    fn get_key(&self) -> String;

    fn get_layout_id(&self) -> String;

    fn get_view_proj_mat(&self) -> glam::Mat4;

    fn is_dirty(&self) -> bool;

    fn get_data(&self) -> Vec<UniformEntry>;

    fn set_aspect_ratio(&mut self, new_aspect: f32);

    fn update_view_proj_mat(&mut self);
}


#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera2DUniform {
    view_proj: [f32; 16],
}

pub struct Camera2D {
    key: String,
    layout_id: String,
    transform: Transform,
    zoom: f32, 
    aspect: f32,
    is_dirty: Cell<bool>,

    view_proj_mat: Mat4,
}

impl Camera for Camera2D {
    fn get_key(&self) -> String {
        self.key.clone()
    }

    fn get_layout_id(&self) -> String {
        self.layout_id.clone()
    }

    fn get_view_proj_mat(&self) -> glam::Mat4 {
        self.view_proj_mat.clone()
    }

    fn is_dirty(&self) -> bool {
        self.is_dirty.get()
    }

    fn get_data(&self) -> Vec<UniformEntry> {
        let camera_uniform = Camera2DUniform {
            view_proj: self.get_view_proj_mat().to_cols_array(),
        };

        vec![UniformEntry { 
            bind_slot: 0, 
            data: bytemuck::bytes_of(&camera_uniform).to_vec() 
        }]
    }

    fn set_aspect_ratio(&mut self, new_aspect: f32) {
        if self.aspect != new_aspect {
            self.aspect = new_aspect;
            self.is_dirty.set(true);
            println!("changed aspect ratio: {}", new_aspect);
        }
    }

    /// update the camera's view-projection matrix
    fn update_view_proj_mat(&mut self) {
        if self.is_dirty() {
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

impl Camera2D {
    pub fn new(key: &str, position: Vec2, zoom: f32, aspect_ratio: f32) -> Self {
        let cam_pos = Vec3::new(position.x, position.y, 0.0);
        let transform = Transform::new(cam_pos, Quat::IDENTITY, Vec3::ONE);
        Self {
            key: key.to_string(),
            layout_id: "camera-2d".to_string(),
            transform,
            zoom,
            aspect: aspect_ratio,
            is_dirty: Cell::new(true),
            view_proj_mat: glam::Mat4::IDENTITY
        }
    }

    pub fn default(key: &str) -> Self {
        let transform = Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
        Self {
            key: key.to_string(),
            layout_id: "camera-2d".to_string(),
            transform,
            zoom: 1.0,
            aspect: 1.0,
            is_dirty: Cell::new(true),
            view_proj_mat: glam::Mat4::IDENTITY
        }
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

    /// Tilt the camera to the left or right relative to local x-axis
    pub fn tilt_local(&mut self, roll: f32) {
        let curr_orient = self.transform.rotation.to_euler(glam::EulerRot::YXZ);
        self.transform.rotate_euler(0.0, 0.0, roll + curr_orient.1);
        self.is_dirty.set(true);
    }

    /// Tilt the camera to the left or right relative to world x-axis
    pub fn tilt_world(&mut self, roll: f32) {
        self.transform.rotate_euler(0.0, 0.0, roll);
        self.is_dirty.set(true);
    }

    /// set the zoom of the camera
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom;
        self.is_dirty.set(true);
    }
}