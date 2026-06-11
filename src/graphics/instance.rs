#![allow(dead_code)]
use std::collections::HashMap;

use crate::graphics::{buffer::BufferBuilder, data_table::{DataTable, DataView, DirtyVec, DynVec}, transform::Transform, vertex::{TransformAttribute, VertexAttribute, VertexAttributeLayout, VertexLayoutBuilder, attr}, wpgu_context::ResourceUpdate};

/// A modifiable template for new entity instances
pub struct InstanceTemplate {
    pub data: HashMap<String, Box<dyn DynVec>>,
}

impl InstanceTemplate {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    /// Get the known attributes of this template
    pub fn attributes(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    /// Add a default value for all known attributes in the template
    pub fn with_defaults(mut self) -> Self {
        for vec in self.data.values_mut() {
            vec.push_default();
        }
        self
    }

    /// Add a transform attribute to this instance
    pub fn with_transform(self, transform: Transform) -> Self {
        self.with_attribute(attr::TRANSFORM, transform)
    }

    /// Update the transform attribute for this instance
    pub fn set_transform(&mut self, transform: Transform) {
        self.set_attribute(attr::TRANSFORM, transform);
    }

    /// Add a component to this instance template
    pub fn with_attribute<D>(mut self, attribute: &str, data: D) -> Self 
    where D: Clone + Default + 'static
    {
        self.set_attribute(attribute, data);
        self
    }

    /// Set a known component's data on this instance template
    pub fn set_attribute<D>(&mut self, attribute: &str, data: D) 
    where D: Clone + Default + 'static
    {
        let vec = DirtyVec::from_vec(vec![data]);
        self.data.insert(attribute.to_string(), Box::new(vec));
    }

    /// set a known component's data to a new DynVec on this instance template.
    /// 
    /// Note: the Implementation must downcast to a DirtyVec<T> for some type T for the template to be valid.
    pub fn set_attribute_vec(&mut self, attribute: &str, vec: Box<dyn DynVec>) {
        self.data.insert(attribute.to_string(), vec);
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
    data: &'a DataTable
}

impl<'a> Instance<'a> {
    pub fn new(data: &'a DataTable, index: usize) -> Self {
        Self { index, data }
    }

    /// Get the properties (known attribute names) of this instance
    pub fn properties(&self) -> Vec<&String> {
        self.data.keys()
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
    pub fn get_attribute<T: Default + Clone+ 'static>(&self, attribute: impl VertexAttribute + 'static) -> Option<&DataView<T>> {
        self.data.properties
            .get(&attribute.name())?
            .downcast_ref::<DirtyVec<T>>()?
            .get(self.index)
    }
}

/// Represents a mutable proxy to an instance in an instance group
pub struct InstanceMut<'a> {
    index: usize,
    data: &'a mut DataTable 
}

impl<'a> InstanceMut<'a> {
    pub fn new(data: &'a mut DataTable, index: usize) -> Self {
        Self { index, data }
    }

    /// Get the properties (known attribute names) of this instance
    pub fn properties(&self) -> Vec<String> {
        let mut properties = Vec::new();
        for (prop, _) in &self.data.properties {
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
    pub fn get_attribute<T: Default + Clone + 'static>(&self, attribute: impl VertexAttribute + 'static) -> Option<&DataView<T>> {
        self.data.properties
            .get(&attribute.name())?
            .downcast_ref::<DirtyVec<T>>()?
            .get(self.index)
    }

    /// Get a reference to an attribute associated with this instance (if no attribute is found, None is returned) 
    pub fn get_attribute_mut<T: Default + Clone + 'static>(&mut self, attribute: impl VertexAttribute + 'static) -> Option<&mut DataView<T>> {
        self.data.properties
            .get_mut(&attribute.name())?
            .downcast_mut::<DirtyVec<T>>()?
            .get_mut(self.index)
    }
}

/// Represents the instances of an entity.
pub struct InstanceGroup {
    label: String,
    instances: DataTable,
    count: usize,
    attributes: VertexAttributeLayout,
}

impl InstanceGroup {
    /// Create a new instance group
    /// 
    /// Note: It is expected that the group will be initialized with attributes with at least init_count amount of data
    pub fn new(init_count: usize, capacity: usize) -> Self {
        Self {
            label: "instances".to_string(),
            instances: DataTable::new(capacity),
            attributes: VertexAttributeLayout::as_instance(),
            count: init_count,
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
        self.count
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
        self.instances.add_property(&attribute.name(),|cap| {
            let mut dirty_vec = DirtyVec::<T>::with_capacity(cap);
            dirty_vec.append(data);

            dirty_vec
        });
        self.attributes.add_attribute(attribute);
    }

    /// Get an instance in the group at the specfied index, if exists.
    pub fn get_instance(&self, index: usize) -> Option<Instance<'_>> {
        if index >= self.count { return None }

        Some(Instance::new(&self.instances, index))
    }

    /// Get a mutable instance in the group at the specfied index, if exists.
    pub fn get_instance_mut(&mut self, index: usize) -> Option<InstanceMut<'_>> {
        if index >= self.count { return None }

        Some(InstanceMut::new(&mut self.instances, index))
    }

    /// Add an instance to the instance group from an instance template
    pub fn add_instance(&mut self, mut instance: InstanceTemplate) {
        for (name, attr) in &mut self.instances.properties {
            if let Some(val) = instance.data.remove(name.as_str()) {
                attr.as_mut().append_from(val.as_ref());
            } else {
                attr.push_default();
            }
        }
        self.count += 1;
    }

    /// Remove the instance at the specified index from the instance group. 
    /// 
    /// This internally uses swap remove, and marks the swapped instance as dirty.
    pub fn remove_instance(&mut self, idx: usize) {
        if idx >= self.count { return; }

        self.instances.swap_remove(idx);
        self.count -= 1;
    }

    /// Remove all instances from the group, but keep their attributes.
    pub fn clear_instances(&mut self) {
        if self.count == 0 { return; }

        self.instances.reset_properties();
        self.count = 0;
    }

    /// Get a reference to the attribute data associated with the provided attribute, if exists
    pub fn get_attribute<T: Default + Clone + 'static>(&self, attribute: &str) -> Option<&DirtyVec<T>> {
        Some(self.instances.get_property::<DirtyVec<T>>(attribute)?)
    }

    /// Get a mutable reference to the attribute data associated with the provided attribute, if exists
    pub fn get_attribute_mut<T: Default + Clone + 'static>(&mut self, attribute: &str) -> Option<&mut DirtyVec<T>> {
        Some(self.instances.get_property_mut::<DirtyVec<T>>(attribute)?)
    }

    /// Get a reference to the instance (vertex) data associated with this group
    pub fn get_instances(&self) -> &DataTable {
        &self.instances
    }

    /// Get a mutable reference to the instance (vertex) data associated with this group
    pub fn get_instances_mut(&mut self) -> &mut DataTable {
        &mut self.instances
    }

    /// Get the vertex layout builder defined by this Geometry
    pub fn get_layout_builder(&self) -> VertexLayoutBuilder {
        self.attributes.get_builder(&self.label)
    }

    /// Get the builder for the instance buffer used by this instance group.
    pub fn get_buffer_builder(&mut self) -> BufferBuilder {
        let instance_bytes = self.attributes.pack(self.count, &mut self.instances);
        let buffer_cap = self.instances.capacity * self.attributes.stride();
        
        BufferBuilder::as_vertex()
            .with_label(&self.label)
            .with_capacity(buffer_cap)
            .with_data(instance_bytes)
    }

    /// Get all updates on the instance data in a vector
    pub fn get_updated(&mut self) -> Vec<ResourceUpdate> {
        self.attributes.get_updated(self.count, &mut self.instances)
    }
}
