#![allow(dead_code)]
use std::cell::Cell;

use crate::graphics::{bind_group::BindGroupLayoutBuilder, buffer::BufferBuilder, presets::{BindingLayout, TextureSampler}, texture::TextureBuilder};

/// Matches against 
pub enum UniformBuilder {
    /// builder that creates a Uniform Buffer
    Buffer(BufferBuilder),
    /// builder that creates a Uniform Texture with a Sampler
    Texture(TextureBuilder, TextureSampler)
}

/// Represents a material, which defines how a mesh should look when rendered
pub trait Material: Clone {
    /// Get the key to this material
    fn get_group_key(&self, mesh_id: u32) -> String;

    /// Get the key-value pairs for buffer data from this material
    fn get_buffers_updated(&self, mesh_id: u32) -> Vec<(String, Vec<u8>)>;

    /// Get the builders for this material mapped by a key (buffers are mesh-specific)
    fn get_uniform_builders(&self, mesh_id: u32) -> Vec<(String, UniformBuilder)>;

    /// Get the bind group layout builder for this material.
    fn get_layout_builder(&self) -> BindGroupLayoutBuilder;

    /// Get the requirements for this mesh as a vector of key-bind slot pairs
    fn get_requirements(&self, mesh_id: u32) -> Vec<(String, u32)>;

    /// Update this material
    fn update(&mut self);
}

/// The structure of the colored sprite uniform data as it lives in the shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColoredSpriteUniform {
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
    fn get_group_key(&self, mesh_id: u32) -> String {
        format!("mesh#{}-colored-sprite", mesh_id)
    }

    fn get_buffers_updated(&self, mesh_id: u32) -> Vec<(String, Vec<u8>)> {
        if self.is_dirty.get() {
            let uniform_data = BufferBuilder::to_padded_vec(
                ColoredSpriteUniform { color: self.color }
            );

            return vec![(self.get_group_key(mesh_id), uniform_data)];
        }
        vec![]
    }

    fn get_requirements(&self, mesh_id: u32) -> Vec<(String, u32)> {
        vec![(self.get_group_key(mesh_id), 0)]
    }

    fn get_uniform_builders(&self, mesh_id: u32) -> Vec<(String, UniformBuilder)> {       
        let uniform_data = ColoredSpriteUniform {
            color: self.color,
        };
        
        let key = self.get_group_key(mesh_id);
        let mat_data = BufferBuilder::to_padded_vec(uniform_data);

        let builder = BufferBuilder::as_uniform( 0)
            .with_label(&key)
            .with_data(mat_data);

        vec![(key, UniformBuilder::Buffer(builder))]
    }

    fn get_layout_builder(&self) -> BindGroupLayoutBuilder {
        BindingLayout::ColoredSprite.get()
    }

    fn update(&mut self) {
        self.is_dirty.set(false);
    }
}
