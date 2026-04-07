use std::cell::Cell;

use crate::graphics::uniform::{UniformEntry, BindGroupBuilder};

/// Represents a material, which defines how a mesh should look when rendered
pub trait Material: Clone {
    /// Get the key to this material
    fn get_key(&self) -> String;

    /// Get the material data as a list of uniform entries - updates the internal dirty flag to false
    fn get_data(&self, model_mat: glam::Mat4) -> Vec<UniformEntry>;

    /// Check if this material has changed since the last uniform request
    fn is_dirty(&self) -> bool;
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColoredSpriteUniform {
    pub model_matrix: [f32; 16],
    pub color: [f32; 4],
}

/// A 2D sprite that can be colored
#[derive(Clone, Debug)]
pub struct ColoredSprite {
    pub key: String,
    pub color: [f32; 4],
    is_dirty: Cell<bool>,
}

impl ColoredSprite {
    pub fn new(color: [f32; 4]) -> Self {
        Self {
            key: "colored-sprite".to_string(),
            color: color,
            is_dirty: Cell::new(true),
        }
    }

    /// Set the color of this material
    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
        self.is_dirty.set(true);
    }
}

impl Material for ColoredSprite {
    fn get_key(&self) -> String {
        self.key.clone()
    }

    fn get_data(&self, model_mat: glam::Mat4) -> Vec<UniformEntry> {
        self.is_dirty.set(false);
        let uniform_data = ColoredSpriteUniform {
            model_matrix: model_mat.to_cols_array(),
            color: self.color,
        };

        vec![UniformEntry { 
            bind_slot: 0, 
            data: BindGroupBuilder::pad_uniform(uniform_data)
        }]
    }

    fn is_dirty(&self) -> bool {
        self.is_dirty.get()
    }
}