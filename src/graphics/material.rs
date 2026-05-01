#![allow(dead_code)]
use std::{cell::Cell, collections::HashMap, sync::atomic::{ AtomicU32, Ordering }};

use crate::graphics::{bind_group::{BindGroupLayoutBuilder, LayoutBindType, LayoutEntry, LayoutVisibility}, wpgu_context::ResourceBinding};

use super::{
    buffer::BufferBuilder, 
    presets::TextureSampler, 
    texture::{TextureBuilder, SamplerBuilder},
    wpgu_context::{ResourceID, ResourceScope}
};

static MAT_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Enum representing different component builders
pub enum UniformBuilder {
    /// builder that creates a Uniform Buffer
    Buffer(BufferBuilder),
    /// builder that creates a Uniform Texture
    Texture(TextureBuilder),
    /// builder that creates a texture sampler
    Sampler(SamplerBuilder)
}

pub trait MaterialComponent {
    /// Get the generic key and scope of this resource
    fn get_id(&self) -> ResourceID;

    /// Get the bind type and visibility of this resource
    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility);

    /// Get the uniform builder for this component
    fn get_uniform_builder(&self) -> UniformBuilder;

    /// Get this component's updated buffer data, if applicable
    fn get_buffer_updated(&self) -> Option<(ResourceID, Vec<u8>)>;
}

/// A high level description of how a mesh should look when rendered
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

    /// Add a component to this material. The component requested is compared against this material's layout map.
    /// If the component is not in the map or the layout builder already has the component's slot occupied, an error is returned.
    pub fn add_component(&mut self, component: impl MaterialComponent + 'static) {
        let slot = self.components.len();
        self.layout_map.insert(component.get_id().key, slot as u32);

        self.components.push(Box::new(component));
    }

    /// Get any buffers that were updated from this material's components as a vector of key-data pairs.
    pub fn get_buffers_updated(&self) -> Vec<(ResourceID, Vec<u8>)> {
        let mut updated: Vec<(ResourceID, Vec<u8>)> = Vec::new();
        for component in &self.components {
            // only components with buffer data need to be considered
            if let Some((mut id, data)) = component.get_buffer_updated() {
                // inject the material's id into the component's namespace
                id.key = self.namespace_component(&id.key);
                updated.push((id, data));
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
    bind_slot: u32,

    color: [f32; 4],
    is_dirty: Cell<bool>,
}

impl ColorComponent {
    pub fn new(label: &str, color: [f32; 4]) -> Self {
        Self {
            label: label.to_string(),
            color,
            bind_slot: 0,
            is_dirty: Cell::new(true),
        }
    }

    /// Set the bind slot for this component (default is 0)
    pub fn with_bind_slot(mut self, slot: u32) -> Self {
        self.bind_slot = slot;
        self
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
            scope: ResourceScope::Entity 
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

    fn get_buffer_updated(&self) -> Option<(ResourceID, Vec<u8>)> {
        if self.is_dirty.get() {
            self.is_dirty.set(false);

            let data = bytemuck::bytes_of(&self.color).to_vec();
            return Some((self.get_id(), data));
        }

        None
    }
}

/// a material component that holds a texture
pub struct TextureComponent {
    label: String,
    path: String,
    bind_slot: u32,
}

impl TextureComponent {
    pub fn new(label: &str, path: &str) -> Self {
        Self {
            label: label.to_string(),
            path: path.to_string(),
            bind_slot: 0
        }
    }

    /// Set the bind slot for this component (default is 0)
    pub fn with_bind_slot(mut self, slot: u32) -> Self {
        self.bind_slot = slot;
        self
    }
}

impl MaterialComponent for TextureComponent {
    fn get_id(&self) -> ResourceID {
        ResourceID { 
            key: self.label.clone(), 
            scope: ResourceScope::Material 
        }
    }

    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility) {
        (LayoutBindType::Texture, LayoutVisibility::Fragment)
    }

    fn get_buffer_updated(&self) -> Option<(ResourceID, Vec<u8>)> {
        None // textures don't have buffers
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        let builder = TextureBuilder::new()
            .with_label(&self.label)
            .with_img_file(&self.path);

        UniformBuilder::Texture(builder)
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
            scope: ResourceScope::Global 
        }
    }

    fn get_vis_type(&self) -> (LayoutBindType, LayoutVisibility) {
        (LayoutBindType::Sampler, LayoutVisibility::Fragment)
    }

    fn get_buffer_updated(&self) -> Option<(ResourceID, Vec<u8>)> {
        None // samplers have no buffers
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        UniformBuilder::Sampler(self.sampler.clone().get())
    }
}
