#![allow(dead_code)]
use std::{cell::Cell, collections::HashMap, sync::atomic::{ AtomicU32, Ordering }};

use super::{
    bind_group::*, 
    buffer::BufferBuilder, 
    presets::TextureSampler, 
    texture::TextureBuilder
};

static MAT_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Matches against 
pub enum UniformBuilder {
    /// builder that creates a Uniform Buffer
    Buffer(BufferBuilder),
    /// builder that creates a Uniform Texture with a Sampler
    Texture(TextureBuilder, TextureSampler)
}

pub trait MaterialComponent {
    /// Get the component specific layout entry information
    fn get_layout_info(&self) -> (LayoutVisibility, LayoutBindType, u32);

    /// Get this component's label
    fn get_label(&self) -> String;

    /// Get the uniform builder for this component
    fn get_uniform_builder(&self) -> UniformBuilder;

    /// Get this component's updated buffer data, if applicable
    fn get_buffer_updated(&self) -> Option<(String, Vec<u8>)>;
}

/// A high level description of how a mesh should look when rendered
pub struct Material {
    id: u32,
    label: String,
    layout_map: HashMap<String, u32>,
    components: Vec<Box<dyn MaterialComponent>>,
    layout_builder: BindGroupLayoutBuilder,
}

impl Material {
    pub fn new(label: &str, layout_map: HashMap<String, u32>) -> Self {
        let id = MAT_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            label: label.to_string(),
            layout_map,
            components: Vec::new(),
            layout_builder: BindGroupLayoutBuilder::new()
        }
    }

    /// Get the unique key of this material (label + id)
    pub fn get_key(&self) -> String {
        format!("{}_{}", self.label, self.id)
    }

    /// Add a component to this material. The component requested is compared against this material's layout map.
    /// If the component is not in the map or the layout builder already has the component's slot occupied, an error is returned.
    pub fn add_component(&mut self, component: impl MaterialComponent + 'static) -> Result<(), String> {
        let comp_label = component.get_label();

        // check if the material supports the component
        let slot = self.layout_map.get(&comp_label)
            .ok_or_else(|| {format!("[Material-{}] Component with label '{}' is not supported.",comp_label, self.get_key())})?;
        
        // check if the layout already has a component mapped to the slot
        if self.layout_builder.has_binding(*slot) {
            return Err(format!("[Material-{}] Layout slot #{} already is occupied.", self.get_key(), slot));
        }

        let mat_comp_key = format!("{}::{}", self.get_key(), comp_label);
        let (visibility, ty, binding) = component.get_layout_info();

        // add the component to the layout builder and internal list
        self.components.push(Box::new(component));
        self.layout_builder.add_entry(LayoutEntry { 
            key: mat_comp_key, binding, visibility, ty
        });

        Ok(())
    }

    /// Get any buffers that were updated from this material's components as a vector of key-data pairs.
    pub fn get_buffers_updated(&self) -> Vec<(String, Vec<u8>)> {
        let material_key = self.get_key();

        let mut updated: Vec<(String, Vec<u8>)> = Vec::new();
        for component in &self.components {
            // only components with buffer data need to be considered
            if let Some(mut buffer_data) = component.get_buffer_updated() {
                // inject the material's id into the component's namespace
                buffer_data.0 = format!("{}::{}", material_key, buffer_data.0);
                updated.push(buffer_data);
            }
        }

        updated
    }

    /// Get the uniforms from this material as vector of key-builder pairs
    pub fn get_uniforms(&self) -> Vec<(String, UniformBuilder)> {
        let mut builders = Vec::new();

        for component in &self.components {
            let mat_comp_key = format!("{}::{}", self.get_key(), component.get_label());
            builders.push((mat_comp_key, component.get_uniform_builder()))
        }

        builders
    }

    /// Get this material's bind group layout builder
    pub fn get_layout(&self) -> BindGroupLayoutBuilder {
        self.layout_builder.clone()
    }
}

/// The structure of the colored sprite uniform data as it lives in the shader
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorUniform {
    pub color: [f32; 4],
}

/// A material component that holds a color
pub struct ColorComponent {
    label: String,
    color: [f32; 4],
    is_dirty: Cell<bool>,

    pub visibility: LayoutVisibility,
    pub binding: u32,
    pub bind_type: LayoutBindType
}

impl ColorComponent {
    pub fn new(label: &str, color: [f32; 4]) -> Self {
        Self {
            label: label.to_string(),
            color, 
            is_dirty: Cell::new(true),
            visibility: LayoutVisibility::VertexFragment,
            bind_type: LayoutBindType::Uniform,
            binding: 0
        }
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
        self.is_dirty.set(false);
    }
}

impl MaterialComponent for ColorComponent {
    fn get_layout_info(&self) -> (LayoutVisibility, LayoutBindType, u32) {
        (LayoutVisibility::Fragment, LayoutBindType::Uniform, 0)
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        let data = BufferBuilder::to_padded_vec(
            ColorUniform { color: self.color }
        );

        let builder = BufferBuilder::as_uniform(0)
            .with_label(&self.label)
            .with_data(data);

        UniformBuilder::Buffer(builder)
    }

    fn get_buffer_updated(&self) -> Option<(String, Vec<u8>)> {
        if self.is_dirty.get() {
            self.is_dirty.set(false);

            let key = self.label.clone();
            let data = bytemuck::bytes_of(&self.color).to_vec();
            return Some((key, data));
        }

        None
    }

    fn get_label(&self) -> String {
        self.label.clone()
    }
}
