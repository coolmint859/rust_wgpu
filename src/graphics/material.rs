use std::{cell::Cell, sync::Arc};

use crate::graphics::{
    traits::ResourceDescriptor
};

/// Represents a single uniform on the gpu
#[derive(Clone, Debug)]
pub struct UniformEntry {
    pub bind_slot: u32,
    pub data: Vec<u8>,
}

/// Represents a group of uniform data to be send to the gpu
#[derive(Clone, Debug)]
pub struct UniformGroup {
    pub key: String,
    pub contents: Vec<UniformEntry>,
    pub bind_layout: Arc<wgpu::BindGroupLayout>,
}

impl ResourceDescriptor for UniformGroup {
    type Key = String;

    fn get_key(&self) -> &String {
        &self.key
    }
}

/// Represents a material, which defines how a mesh should look when rendered
pub trait Material {
    /// Check if any material data has changed, and if so, return them as uniform entries.
    fn diff(&self) -> Option<Vec<UniformEntry>>;

    /// Get the key to this material
    fn get_key(&self) -> String;

    /// Get the material data as a list of uniform entries
    fn entries(&self) -> Vec<UniformEntry>;
}

/// A 2D sprite that can be colored
#[derive(Clone, Debug)]
pub struct ColoredSprite {
    pub key: String,
    pub color: [f32; 4],
    is_dirty: Cell<bool>,
}

impl ColoredSprite {
    pub fn new(color: [f32; 4]) -> Arc<Self> {
        Arc::new(Self {
            key: "colored-sprite".to_string(),
            color: color,
            is_dirty: Cell::new(true),
        })
    }
}

impl Material for ColoredSprite {
    fn diff(&self) -> Option<Vec<UniformEntry>> {
        if self.is_dirty.get() {
            self.is_dirty.set(false);
            Some(self.entries())
        } else {
            None
        }
    }

    fn get_key(&self) -> String {
        self.key.clone()
    }

    fn entries(&self) -> Vec<UniformEntry> {
        vec![UniformEntry { bind_slot: 1, data: bytemuck::cast_slice(&self.color).to_vec() }]
    }
}