#![allow(dead_code)]
use std::{cell::Cell, collections::HashMap, sync::atomic::{ AtomicU32, Ordering }};

use crate::graphics::wpgu_context::{ResourceBinding, ResourceID, ResourceScope};

use super::{
    bind_group::*, 
    buffer::BufferBuilder, 
    presets::TextureSampler, 
    texture::{TextureBuilder, SamplerBuilder}
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
    /// Get the component id and binding
    fn get_binding(&self) -> ResourceBinding;

    /// Get the layout entry respresented by this component
    fn get_entry(&self) -> LayoutEntry;

    /// Get the uniform builder for this component
    fn get_uniform_builder(&self) -> UniformBuilder;

    /// Get this component's updated buffer data, if applicable
    fn get_buffer_updated(&self) -> Option<(ResourceID, Vec<u8>)>;
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
            layout_builder: BindGroupLayoutBuilder::new().with_label("material-uniforms")
        }
    }

    /// Get the unique key of this material (label + id)
    pub fn get_key(&self) -> String {
        format!("{}_{}", self.label, self.id)
    }

    /// Add a component to this material. The component requested is compared against this material's layout map.
    /// If the component is not in the map or the layout builder already has the component's slot occupied, an error is returned.
    pub fn add_component(&mut self, component: impl MaterialComponent + 'static) -> Result<(), String> {
        let comp_binding = component.get_binding();
        let comp_key = comp_binding.id.key;

        // check if the material supports the component
        let slot = self.layout_map.get(&comp_key)
            .ok_or_else(|| {format!("[Material-{}] Component with label '{}' is not supported.", comp_key, self.get_key())})?;
        
        // check if the layout already has a component mapped to the slot
        if self.layout_builder.has_binding(*slot) {
            return Err(format!("[Material-{}] Layout slot #{} already is occupied.", self.get_key(), slot));
        }

        // add the component to the layout builder and internal list
        self.layout_builder.add_entry(component.get_entry());
        self.components.push(Box::new(component));

        Ok(())
    }

    /// Get any buffers that were updated from this material's components as a vector of key-data pairs.
    pub fn get_buffers_updated(&self) -> Vec<(ResourceID, Vec<u8>)> {
        let mut updated: Vec<(ResourceID, Vec<u8>)> = Vec::new();
        for component in &self.components {
            // only components with buffer data need to be considered
            if let Some((mut comp_id, data)) = component.get_buffer_updated() {
                // inject the material's id into the component's namespace
                comp_id.key = self.namespace_component(&comp_id.key);
                updated.push((comp_id, data));
            }
        }

        updated
    }

    /// Get the uniforms from this material as vector of key-builder pairs
    pub fn get_uniforms(&self) -> Vec<(ResourceBinding, UniformBuilder)> {
        let mut builders = Vec::new();

        for component in &self.components {
            let mut comp_binding = component.get_binding();
            let named_comp = match comp_binding.id.scope {
                ResourceScope::Global => comp_binding,
                _ => { // Material or Entity scope
                    comp_binding.id.key = self.namespace_component(&comp_binding.id.key);
                    comp_binding
                }
            };

            builders.push((named_comp, component.get_uniform_builder()))
        }

        builders
    }

    /// Get this material's bind group layout builder
    pub fn get_layout(&self) -> BindGroupLayoutBuilder {
        self.layout_builder.clone()
    }

    /// Namespace a component's resource id to this material
    fn namespace_component(&self, comp_key: &String) -> String {
        format!("{}::{}", self.get_key(), comp_key)
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

    /// Get the unique id of this component
    fn get_id(&self) -> ResourceID {
        ResourceID {
            key: self.label.clone(),
            scope: ResourceScope::Entity,
        }
    }
}

impl MaterialComponent for ColorComponent {
    fn get_entry(&self) -> LayoutEntry {
        LayoutEntry { 
            binding: self.bind_slot, 
            visibility: LayoutVisibility::Fragment, 
            ty: LayoutBindType::Uniform 
        }
    }

    fn get_binding(&self) -> ResourceBinding {
        ResourceBinding { 
            id: self.get_id(), 
            slot: self.bind_slot
        }
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

    /// Get the unique id of this component
    fn get_id(&self) -> ResourceID {
        ResourceID {
            key: self.label.clone(),
            scope: ResourceScope::Material,
        }
    }
}

impl MaterialComponent for TextureComponent {
    fn get_binding(&self) -> ResourceBinding {
        ResourceBinding { 
            id: self.get_id(), 
            slot: self.bind_slot 
        }
    }

    fn get_entry(&self) -> LayoutEntry {
        LayoutEntry { 
            binding: self.bind_slot, 
            visibility: LayoutVisibility::Fragment, 
            ty: LayoutBindType::Texture 
        }
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
    sampler: TextureSampler,
    bind_slot: u32,
}

impl SamplerComponent {
    pub fn new(sampler: TextureSampler) -> Self {
        Self {
            sampler,
            bind_slot: 0
        }
    }

    /// Set the bind slot for this component (default is 0)
    pub fn with_bind_slot(mut self, slot: u32) -> Self {
        self.bind_slot = slot;
        self
    }

    /// Get the unique id of this component
    fn get_id(&self) -> ResourceID {
        ResourceID {
            key: self.sampler.clone().as_key(),
            scope: ResourceScope::Global,
        }
    }
}

impl MaterialComponent for SamplerComponent {
    fn get_binding(&self) -> ResourceBinding {
        ResourceBinding { 
            id: self.get_id(), 
            slot: self.bind_slot 
        }
    }

    fn get_entry(&self) -> LayoutEntry {
        LayoutEntry { 
            binding: self.bind_slot, 
            visibility: LayoutVisibility::Fragment, 
            ty: LayoutBindType::Sampler 
        }
    }

    fn get_buffer_updated(&self) -> Option<(ResourceID, Vec<u8>)> {
        None // samplers have no buffers
    }

    fn get_uniform_builder(&self) -> UniformBuilder {
        UniformBuilder::Sampler(self.sampler.clone().get())
    }
}