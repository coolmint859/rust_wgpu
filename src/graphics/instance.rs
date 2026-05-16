#![allow(dead_code)]
use std::collections::HashMap;

use crate::graphics::{buffer::BufferBuilder, data_utils::{DataView, DirtyVec, DynHashMap, DynVector, PackingUtils}, transform::Transform, vertex::{TINT_LOC, TRANSFORM_LOC, VertexComponent, VertexLayoutBuilder}, wpgu_context::ResourceUpdate};

pub const TRANSFORM_ATTR: &str = "transform";
pub const TINT_ATTR: &str = "tint";

/// A modifiable template for new entity instances
pub struct InstanceTemplate {
    pub values: HashMap<String, Box<dyn DynVector>>
}

impl InstanceTemplate {
    pub fn new() -> Self {
        Self { values: HashMap::new() }
    }

    /// Add a transform attribute to this instance
    pub fn with_transform(self, transform: Transform) -> Self {
        self.with_attr::<Transform>(TRANSFORM_ATTR, transform)
    }

    /// Update the transform attribute for this instance
    pub fn set_transform(&mut self, transform: Transform) {
        self.set_attr::<Transform>(TRANSFORM_ATTR, transform);
    }

    /// Add an attribute to this instance
    pub fn with_attr<T: Clone + Default + 'static>(mut self, name: &str, value: T) -> Self {
        let vec = DirtyVec::from_vec(vec![value]);
        self.values.insert(name.to_string(), Box::new(vec));
        self
    }

    /// Update an attribute to this instance
    pub fn set_attr<T: Clone + Default + 'static>(&mut self, name: &str, value: T) {
        let vec = DirtyVec::from_vec(vec![value]);
        self.values.insert(name.to_string(), Box::new(vec));
    }
}

impl Clone for InstanceTemplate {
    fn clone(&self) -> Self {
        let mut new_values = HashMap::new();
        for (name, attr) in &self.values {
            new_values.insert(name.clone(), attr.as_ref().clone_box());
        }

        Self { values: new_values }
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
        self.get_attribute::<Transform>(TRANSFORM_ATTR)
    }

    /// Get a reference to this instance's transform
    /// 
    /// ## Panics
    /// If the instance is missing a transform attribute
    pub fn transform(&self) -> &DataView<Transform> {
        self.get_attribute::<Transform>(TRANSFORM_ATTR)
            .expect("Expected the instance to have a transform attribute, but none was found.")
    }

    /// Get a reference to an attribute associated with this instance (if no attribute is found, None is returned) 
    pub fn get_attribute<T: 'static>(&self, attr_name: &str) -> Option<&DataView<T>> {
        let attributes = self.data.attributes.get_vec::<DirtyVec<T>>(attr_name)?;
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
        self.get_attribute::<Transform>(TRANSFORM_ATTR)
    }

    /// Get a reference to this instance's transform
    /// 
    /// ## Panics
    /// If the instance is missing a transform attribute
    pub fn transform(&self) -> &DataView<Transform> {
        self.get_attribute::<Transform>(TRANSFORM_ATTR)
            .expect("Expected the instance to have a transform attribute, but none was found.")
    }

    /// Get a mutable reference to this instance's transform
    /// 
    /// ## Panics
    /// If the instance is missing a transform attribute
    pub fn transform_mut(&mut self) -> &mut DataView<Transform> {
        self.get_attribute_mut::<Transform>(TRANSFORM_ATTR)
            .expect("Expected the instance to have a transform attribute, but none was found.")
    }

    /// Get a mutable reference to this instance's transform
    pub fn get_transform_mut(&mut self) -> Option<&mut DataView<Transform>> {
        self.get_attribute_mut::<Transform>(TRANSFORM_ATTR)
    }

    /// Get a reference to an attribute associated with this instance (if no attribute is found, None is returned) 
    pub fn get_attribute<T: 'static>(&self, attr_name: &str) -> Option<&DataView<T>> {
        let attributes = self.data.attributes.get_vec::<DirtyVec<T>>(attr_name)?;
        Some(&attributes.inner[self.index])
    }

    /// Get a reference to an attribute associated with this instance (if no attribute is found, None is returned) 
    pub fn get_attribute_mut<T: 'static>(&mut self, attr_name: &str) -> Option<&mut DataView<T>> {
        let attributes = self.data.attributes.get_vec_mut::<DirtyVec<T>>(attr_name)?;
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

    /// add attribute data to this instance group
    pub fn with_attr(mut self, key: &str, data: impl DynVector) -> Self {
        self.attributes.map.insert(key.to_string(), Box::new(data));
        self
    }
}

/// Represents the instances of an entity.
pub struct InstanceGroup {
    label: String,
    instances: InstanceData,
    components: Vec<Box<dyn VertexComponent>>,
}

impl InstanceGroup {
    pub fn new(instances: InstanceData) -> Self {
        Self {
            label: "instances".to_string(),
            instances, 
            components: Vec::new() 
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
            if let Some(val) = instance.values.remove(name.as_str()) {
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
    pub fn get_attribute<T: 'static>(&self, attr_name: &str) -> Option<&Vec<DataView<T>>> {
        let attributes = self.instances.attributes.get_vec::<DirtyVec<T>>(attr_name)?;
        Some(&attributes.inner)
    }

    /// Get a mutable reference to the attribute data associated with the provided attribute name, if exists
    pub fn get_attribute_mut<T: 'static>(&mut self, attr_name: &str) -> Option<&mut Vec<DataView<T>>> {
        self.instances.attributes.get_vec_mut(attr_name)
    }

    /// Add a vertex component to this geometry
    pub fn with_component(mut self, component: impl VertexComponent + 'static) -> Self {
        self.add_component(component);
        self
    }

    /// Add a vertex component to this geometry
    pub fn add_component(&mut self, component: impl VertexComponent + 'static) {
        self.components.push(Box::new(component));
    }

    /// Get the vertex layout builder defined by this Geometry
    pub fn get_layout_builder(&self) -> VertexLayoutBuilder {
        PackingUtils::layout_builder(wgpu::VertexStepMode::Instance, &self.components)
    }

    /// Get the builder for the instance buffer used by this instance group.
    pub fn get_buffer_builder(&self) -> BufferBuilder {
        let instance_bytes = PackingUtils::pack(self.instances.count, &self.instances.attributes, &self.components);
        let buffer_cap = self.instances.capacity * PackingUtils::instance_stride(&self.components);
        
        BufferBuilder::as_vertex()
            .with_label(&self.label)
            .with_capacity(buffer_cap)
            .with_data(instance_bytes)
    }

    /// Get all updates on the instance data in a vector
    pub fn get_updated(&self) -> Vec<ResourceUpdate> {
        PackingUtils::get_updated(self.instances.count, &self.instances.attributes, &self.components)
    }
}

/// Instance component representing the Transform attribute
pub struct TransformComponent;

impl VertexComponent for TransformComponent {
    fn attribute(&self) -> &'static str { TRANSFORM_ATTR }
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

/// Instance component representing the tint attribute
pub struct TintComponent;

impl VertexComponent for TintComponent {
    fn attribute(&self) -> &'static str { TINT_ATTR }
    fn location(&self) -> u32 { TINT_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(tints) = attributes.get_vec::<DirtyVec<glam::Vec4>>(TINT_ATTR) {
            buffer.extend_from_slice(bytemuck::bytes_of(&*tints.inner[idx]));
        }
    }
}