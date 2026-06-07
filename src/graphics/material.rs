#![allow(dead_code)]
use std::{cell::Cell, collections::HashMap, sync::atomic::{ AtomicU32, Ordering }};

use crate::graphics::{bind_group::{BindGroupLayoutBuilder, LayoutBindType, LayoutEntry, LayoutVisibility}, wpgu_context::{ResourceBinding, ResourceType, ResourceUpdate}};

use super::{
    buffer::BufferBuilder, 
    presets::TextureSampler, 
    texture::{TextureBuilder, SamplerBuilder},
    wpgu_context::{ResourceID, ResourceScope}
};

static MAT_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Enum representing different component builders
pub enum UniformBuilder {
    /// Builder that creates a uniform buffer
    Buffer(BufferBuilder),
    /// Builder that creates a standard rgba texture
    Texture(TextureBuilder),
    /// Builder that creates a texture sampler
    Sampler(SamplerBuilder),
    /// Builder is handled by an external system (for example, Fonts)
    PreAllocated
}

/// Represents uniforms stored in a bind group, attached to a material
pub trait MaterialComponent {
    /// Get the generic key and scope of this resource
    fn get_id(&self) -> ResourceID;

    /// Get the bind type and visibility of this resource
    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility);

    /// Get the uniform builder for this component
    fn get_uniform_builder(&self) -> UniformBuilder;

    /// Get this component's updated buffer data, if applicable
    fn get_updated(&self) -> Option<ResourceUpdate> { None }
}

/// A high level description of how a primitive should look when rendered
pub struct Material {
    id: u32,
    label: String,
    components: Vec<Box<dyn MaterialComponent>>,
    layout_map: HashMap<String, u32>
}

impl Material {
    pub fn new(label: &str) -> Self {
        let id = MAT_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            label: label.to_string(),
            components: Vec::new(),
            layout_map: HashMap::new()
        }
    }

    /// Get the unique key of this material (label + id)
    pub fn get_key(&self) -> String {
        format!("{}_{}", self.label, self.id)
    }

    /// Add a component to this material.
    pub fn add_component(&mut self, component: impl MaterialComponent + 'static) {
        let slot = self.components.len();
        self.layout_map.insert(component.get_id().key, slot as u32);

        self.components.push(Box::new(component));
    }

    /// Get any buffers that were updated from this material's components as a vector of key-data pairs.
    pub fn get_updated(&self) -> Vec<(ResourceID, ResourceUpdate)> {
        let mut updated: Vec<(ResourceID, ResourceUpdate)> = Vec::new();
        for component in &self.components {
            // only components with buffer data need to be considered
            if let Some(update) = component.get_updated() {
                let mut id = component.get_id();
                // inject the material's id into the component's namespace
                id.key = self.namespace_component(&id.key);

                updated.push((id, update));
            }
        }

        updated
    }
    
    /// Get the 'signature' of this material in the form of a bind group layout builder
    pub fn get_layout_builder(&self) -> BindGroupLayoutBuilder {
        let mut builder = BindGroupLayoutBuilder::new().with_label(&self.label);

        let mut binding = 0;
        for component in &self.components {
            let (ty, visibility) = component.get_vis_type();
            builder.add_entry(LayoutEntry { binding, visibility, ty });
            binding += 1;
        }

        builder
    }

    /// Get the uniforms from this material as vector of binding-builder pairs
    pub fn get_uniforms(&self) -> Vec<(ResourceBinding, UniformBuilder)> {
        let mut builders = Vec::new();

        let mut slot = 0;
        for component in &self.components {
            let mut id = component.get_id();
            id.key = match id.scope {
                ResourceScope::Global => id.key,
                _ => self.namespace_component(&id.key)
            };

            let binding = ResourceBinding { id, slot };

            builders.push((binding, component.get_uniform_builder()));
            slot += 1;
        }

        builders
    }

    /// Namespace a component's resource id to this material
    fn namespace_component(&self, comp_label: &String) -> String {
        format!("{}::{}", self.get_key(), comp_label)
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
}

impl ColorComponent {
    pub fn new(label: &str, color: [f32; 4]) -> Self {
        Self {
            label: label.to_string(),
            color,
            is_dirty: Cell::new(true),
        }
    }

    /// set the color
    pub fn set_color(&mut self, color: [f32; 4]) {
        self.color = color;
        self.is_dirty.set(false);
    }
}

impl MaterialComponent for ColorComponent {
    fn get_id(&self) -> ResourceID {
        ResourceID { 
            key: self.label.clone(), 
            scope: ResourceScope::Primitive,
            r_type: ResourceType::Uniform,
        }
    }

    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility) {
        (LayoutBindType::Uniform, LayoutVisibility::Fragment)
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        let builder = BufferBuilder::as_uniform()
            .with_label(&self.label)
            .with_data_from_struct(ColorUniform { color: self.color });

        UniformBuilder::Buffer(builder)
    }

    fn get_updated(&self) -> Option<ResourceUpdate> {
        if self.is_dirty.get() {
            self.is_dirty.set(false);

            let data = bytemuck::bytes_of(&self.color).to_vec();
            return Some(ResourceUpdate { data, offset: 0 });
        }
        None
    }
}

/// a material component that holds a texture
pub struct TextureComponent {
    label: String,
    path: String,
}

impl TextureComponent {
    pub fn new(label: &str, path: &str,) -> Self {
        Self {
            label: label.to_string(),
            path: path.to_string(),
        }
    }
}

impl MaterialComponent for TextureComponent {
    fn get_id(&self) -> ResourceID {
        ResourceID { 
            key: self.path.clone(), 
            scope: ResourceScope::Material,
            r_type: ResourceType::Texture,
        }
    }

    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility) {
        (LayoutBindType::Texture, LayoutVisibility::Fragment)
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        let builder = TextureBuilder::new()
            .with_label(&self.label)
            .with_img_file(&self.path);

        UniformBuilder::Texture(builder)
    }
}

/// A material component that holds a font (texture) atlas
pub struct FontComponent {
    label: String,
    id: ResourceID,
}

impl FontComponent {
    pub fn new(label: &str, id: ResourceID) -> Self {
        Self {
            label: label.to_string(),
            id,
        }
    }
}

impl MaterialComponent for FontComponent {
    fn get_id(&self) -> ResourceID {
        self.id.clone()
    }

    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility) {
        (LayoutBindType::Texture, LayoutVisibility::Fragment)
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        UniformBuilder::PreAllocated
    }
}

pub struct SamplerComponent {
    label: String,
    sampler: TextureSampler,
}

impl SamplerComponent {
    pub fn new(sampler: TextureSampler) -> Self {
        Self { label: sampler.label(), sampler }
    }
}

impl MaterialComponent for SamplerComponent {
    fn get_id(&self) -> ResourceID {
        ResourceID { 
            key: self.label.clone(), 
            scope: ResourceScope::Global,
            r_type: ResourceType::Sampler,
        }
    }

    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility) {
        (LayoutBindType::Sampler, LayoutVisibility::Fragment)
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        UniformBuilder::Sampler(self.sampler.clone().get())
    }
}
