#![allow(dead_code)]
use std::collections::HashMap;

use crate::graphics::{buffer::BufferBuilder, data_utils::{DataView, DirtyVec, DynDirtyVec, DynHashMap, PackingUtils}, transform::Transform, vertex::{TINT_LOC, TRANSFORM_LOC, UV_BOUNDS_LOC, VertexAttribute, VertexLayoutBuilder}, wpgu_context::ResourceUpdate};

pub const TRANSFORM_ATTR: &str = "transform";
pub const TINT_ATTR: &str = "tint";
pub const UV_BOUNDS_ATTR: &str = "uv_bounds";

/// A modifiable template for new entity instances
pub struct InstanceTemplate {
    pub data: HashMap<String, Box<dyn DynDirtyVec>>,
}

impl InstanceTemplate {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    /// Add a transform attribute to this instance
    pub fn with_transform(self, transform: Transform) -> Self {
        self.with_attribute(TransformAttribute, transform)
    }

    /// Update the transform attribute for this instance
    pub fn set_transform(&mut self, transform: Transform) {
        self.set_attribute(TransformAttribute, transform);
    }

    /// Add a component to this instance template
    pub fn with_attribute<V, D>(mut self, attributes: V, data: D) -> Self 
    where 
        V: VertexAttribute + 'static, 
        D: Clone + Default + 'static
    {
        self.set_attribute(attributes, data);
        self
    }

    /// Set a known component's data on this instance template
    pub fn set_attribute<V, D>(&mut self, attributes: V, data: D) 
    where 
        V: VertexAttribute + 'static, 
        D: Clone + Default + 'static
    {
        let vec = DirtyVec::from_vec(vec![data]);
        self.data.insert(attributes.name().to_string(), Box::new(vec));
    }
}

impl Clone for InstanceTemplate {
    fn clone(&self) -> Self {
        let mut new_values = HashMap::new();
        for (name, attr) in &self.data {
            new_values.insert(name.clone(), attr.as_ref().clone_box());
        }

        Self { data: new_values }
    }
}

/// Represents a proxy to an instance in an instance group
pub struct Instance<'a> {
    index: usize,
    data: &'a InstanceData
}

impl<'a> Instance<'a> {
    pub fn new(data: &'a InstanceData, index: usize) -> Self {
        Self { index, data }
    }

    /// Get the properties (known attribute names) of this instance
    pub fn properties(&self) -> Vec<String> {
        let mut properties = Vec::new();
        for (prop, _) in &self.data.attributes.map {
            properties.push(prop.clone())
        }

        properties
    }

    /// Get a reference to this instance's transform
    pub fn get_transform(&self) -> Option<&DataView<Transform>> {
        self.get_attribute::<Transform>(TransformAttribute)
    }

    /// Get a reference to this instance's transform
    /// 
    /// ## Panics
    /// If the instance is missing a transform attribute
    pub fn transform(&self) -> &DataView<Transform> {
        self.get_attribute::<Transform>(TransformAttribute)
            .expect("Expected the instance to have a transform attribute, but none was found.")
    }

    /// Get a reference to an attribute associated with this instance (if no attribute is found, None is returned) 
    pub fn get_attribute<T: 'static>(&self, attribute: impl VertexAttribute + 'static) -> Option<&DataView<T>> {
        let attributes = self.data.attributes.get_vec::<DirtyVec<T>>(attribute.name())?;
        Some(&attributes.inner[self.index])
    }
}

/// Represents a mutable proxy to an instance in an instance group
pub struct InstanceMut<'a> {
    index: usize,
    data: &'a mut InstanceData 
}

impl<'a> InstanceMut<'a> {
    pub fn new(data: &'a mut InstanceData, index: usize) -> Self {
        Self { index, data }
    }

    /// Get the properties (known attribute names) of this instance
    pub fn properties(&self) -> Vec<String> {
        let mut properties = Vec::new();
        for (prop, _) in &self.data.attributes.map {
            properties.push(prop.clone())
        }

        properties
    }

    /// Get a reference to this instance's transform
    pub fn get_transform(&self) -> Option<&DataView<Transform>> {
        self.get_attribute::<Transform>(TransformAttribute)
    }

    /// Get a reference to this instance's transform
    /// 
    /// ## Panics
    /// If the instance is missing a transform attribute
    pub fn transform(&self) -> &DataView<Transform> {
        self.get_attribute::<Transform>(TransformAttribute)
            .expect("Expected the instance to have a transform attribute, but none was found.")
    }

    /// Get a mutable reference to this instance's transform
    /// 
    /// ## Panics
    /// If the instance is missing a transform attribute
    pub fn transform_mut(&mut self) -> &mut DataView<Transform> {
        self.get_attribute_mut::<Transform>(TransformAttribute)
            .expect("Expected the instance to have a transform attribute, but none was found.")
    }

    /// Get a mutable reference to this instance's transform
    pub fn get_transform_mut(&mut self) -> Option<&mut DataView<Transform>> {
        self.get_attribute_mut::<Transform>(TransformAttribute)
    }

    /// Get a reference to an attribute associated with this instance (if no attribute is found, None is returned) 
    pub fn get_attribute<T: 'static>(&self, attribute: impl VertexAttribute + 'static) -> Option<&DataView<T>> {
        let attributes = self.data.attributes.get_vec::<DirtyVec<T>>(attribute.name())?;
        Some(&attributes.inner[self.index])
    }

    /// Get a reference to an attribute associated with this instance (if no attribute is found, None is returned) 
    pub fn get_attribute_mut<T: 'static>(&mut self, attribute: impl VertexAttribute + 'static) -> Option<&mut DataView<T>> {
        let attributes = self.data.attributes.get_vec_mut::<DirtyVec<T>>(attribute.name())?;
        Some(&mut attributes.inner[self.index])
    }
}

/// Contains the raw data used in an instance buffer
pub struct InstanceData {
    /// The instance attributes
    pub attributes: DynHashMap,
    /// The number of active instances
    pub count: usize,
    /// the capacity of the instance group
    pub capacity: usize,
}

impl InstanceData {
    pub fn new(count: usize, capacity: usize) -> Self {
        Self {
            attributes: DynHashMap::new(),
            count,
            capacity
        }
    }

    /// Add attribute data to this instance group
    pub fn with_attr(mut self, key: &str, data: impl DynDirtyVec) -> Self {
        self.add_attr(key, data);
        self
    }

    /// Add attribute data to this instance group
    pub fn add_attr(&mut self, key: &str, data: impl DynDirtyVec) {
        self.attributes.map.insert(key.to_string(), Box::new(data));
    }
}

/// Represents the instances of an entity.
pub struct InstanceGroup {
    label: String,
    instances: InstanceData,
    attributes: Vec<Box<dyn VertexAttribute>>,
}

impl InstanceGroup {
    /// Create a new instance group
    /// 
    /// Note: It is expected that the group will be initialized with attributes with at least init_count amount of data
    pub fn new(init_count: usize, capacity: usize) -> Self {
        Self {
            label: "instances".to_string(),
            instances: InstanceData::new(init_count, capacity),
            attributes: Vec::new()
        }
    }

    /// Set the label for this instance group
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Get the label of this instance group
    pub fn get_label(&self) -> String {
        self.label.clone()
    }

    /// Get the current number of instances in the group
    pub fn count(&self) -> usize {
        self.instances.count
    }

    /// Add an attribute to the instance group
    pub fn with_attribute<V, T>(mut self, attribute: V, data: Vec<T>) -> Self
    where 
        V: VertexAttribute + 'static,
        T: Clone + Default + 'static,
    {
        self.add_attribute(attribute, data);
        self
    }

    /// Add an attribute to the instance group
    pub fn add_attribute<V, T>(&mut self, attribute: V, data: Vec<T>)
    where 
        V: VertexAttribute + 'static,
        T: Clone + Default + 'static,
    {
        let dirty_vec = DirtyVec::<T>::from_vec(data);
        self.instances.add_attr(attribute.name(), dirty_vec);
        self.attributes.push(Box::new(attribute));
    }

    /// Get an instance in the group at the specfied index, if exists.
    pub fn get_instance(&self, index: usize) -> Option<Instance<'_>> {
        if index >= self.instances.count { return None }

        Some(Instance::new(&self.instances, index))
    }

    /// Get a mutable instance in the group at the specfied index, if exists.
    pub fn get_instance_mut(&mut self, index: usize) -> Option<InstanceMut<'_>> {
        if index >= self.instances.count { return None }

        Some(InstanceMut::new(&mut self.instances, index))
    }

    /// Add an instance to the instance group from an instance template
    pub fn add_instance(&mut self, mut instance: InstanceTemplate) {
        for (name, attr) in &mut self.instances.attributes.map {
            if let Some(val) = instance.data.remove(name.as_str()) {
                attr.as_mut().append_from(val.as_ref());
            } else {
                attr.push_default();
            }
        }
        self.instances.count += 1;
    }

    /// Remove the instance at the specified index from the instance group. 
    /// 
    /// This internally uses swap remove, and marks the swapped instance as dirty.
    pub fn remove_instance(&mut self, idx: usize) {
        if idx >= self.instances.count { return; }

        for attr in self.instances.attributes.map.values_mut() {
            attr.swap_remove(idx);
            attr.mark_dirty(idx);
        }
        self.instances.count -= 1;
    }

    /// Get a reference to the attribute data associated with the provided attribute name, if exists
    pub fn get_attribute<T: 'static>(&self, attribute: impl VertexAttribute + 'static) -> Option<&Vec<DataView<T>>> 
    {
        let attributes = self.instances.attributes.get_vec::<DirtyVec<T>>(attribute.name())?;
        Some(&attributes.inner)
    }

    /// Get a mutable reference to the attribute data associated with the provided attribute name, if exists
    pub fn get_attribute_mut<T: 'static>(&mut self, attribute: impl VertexAttribute + 'static) -> Option<&mut Vec<DataView<T>>> 
    {
        let attributes = self.instances.attributes.get_vec_mut::<DirtyVec<T>>(attribute.name())?;
        Some(&mut attributes.inner)
    }

    /// Get the vertex layout builder defined by this Geometry
    pub fn get_layout_builder(&self) -> VertexLayoutBuilder {
        PackingUtils::layout_builder(wgpu::VertexStepMode::Instance, &self.attributes)
    }

    /// Get the builder for the instance buffer used by this instance group.
    pub fn get_buffer_builder(&self) -> BufferBuilder {
        let instance_bytes = PackingUtils::pack(self.instances.count, &self.instances.attributes, &self.attributes);
        let buffer_cap = self.instances.capacity * PackingUtils::instance_stride(&self.attributes);
        
        BufferBuilder::as_vertex()
            .with_label(&self.label)
            .with_capacity(buffer_cap)
            .with_data(instance_bytes)
    }

    /// Get all updates on the instance data in a vector
    pub fn get_updated(&self) -> Vec<ResourceUpdate> {
        PackingUtils::get_updated(self.instances.count, &self.instances.attributes, &self.attributes)
    }
}

/// Instance attribute for transforms (world matrix)
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct TransformAttribute;

impl VertexAttribute for TransformAttribute {
    fn name(&self) -> &'static str { TRANSFORM_ATTR }
    fn location(&self) -> u32 { TRANSFORM_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
    fn attr_count(&self) -> u32 { 4 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(transforms) = attributes.get_vec::<DirtyVec<Transform>>(TRANSFORM_ATTR) {
            let world_mat = transforms.inner[idx].world_matrix();
            buffer.extend_from_slice(bytemuck::bytes_of(&world_mat));
        }
    }
}

/// Instance attribute for tints
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct TintAttribute;

impl VertexAttribute for TintAttribute {
    fn name(&self) -> &'static str { TINT_ATTR }
    fn location(&self) -> u32 { TINT_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(tints) = attributes.get_vec::<DirtyVec<glam::Vec4>>(TINT_ATTR) {
            buffer.extend_from_slice(bytemuck::bytes_of(&*tints.inner[idx]));
        }
    }
}

/// Instance attribute for uv offsets and scales for use in a spritesheet
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct UVBoundsAttribute;

impl VertexAttribute for UVBoundsAttribute {
    fn name(&self) -> &'static str { UV_BOUNDS_ATTR }
    fn location(&self) -> u32 { UV_BOUNDS_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(uv_bounds) = attributes.get_vec::<DirtyVec<glam::Vec4>>(UV_BOUNDS_ATTR) {
            buffer.extend_from_slice(bytemuck::bytes_of(&*uv_bounds.inner[idx]));
        }
    }
}