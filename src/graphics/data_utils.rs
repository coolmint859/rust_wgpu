#![allow(dead_code)]
use std::{any::Any, cell::Cell, ops::{Deref, DerefMut}};

use glam::*;

use crate::graphics::{vertex::{VertexAttribute, VertexData, VertexLayoutBuilder}, wpgu_context::ResourceUpdate};

/// Represents a type erased vector that stores generic data
pub trait DynVec: Any {
    /// Push a default value into the internal vector
    fn push_default(&mut self);
    /// Swap the last element in the vector with the one at the index, and then remove the last element.
    fn swap_remove(&mut self, idx: usize);
    /// Clone the values of the vector into a new one.
    fn clone_box(&self) -> Box<dyn DynDirtyVec>;
    /// Append data from another vector into this one.
    fn append_from(&mut self, other: &dyn DynDirtyVec);

    /// Convert the data store into it's Any type
    fn as_any_ref<'a>(&'a self) -> &'a dyn Any;
    /// Convert the data store into it's Any type as mutable
    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn Any;
}

impl dyn DynVec {
    /// Downcast this dynamic vector into a reference of the concrete vector type
    pub fn downcast_ref<T: 'static>(&self) -> Option<&Vec<T>> {
        self.as_any_ref().downcast_ref::<Vec<T>>()
    }

    /// Downcast this dynamic vector into a mutable reference of the concrete vector type
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut Vec<T>> {
        self.as_any_mut().downcast_mut::<Vec<T>>()
    }
}

/// Represents a type erased vector that stores values wrapped in a DataView<T>
pub trait DynDirtyVec: Any {
    /// Check if data at the specified index has changed
    fn is_dirty(&self, idx: usize) -> bool;
    /// Mark the data at the specified index as clean
    fn mark_clean(&self, idx: usize);
    /// Mark the data at the specified index as dirty
    fn mark_dirty(&self, idx: usize);

    /// Push a default value into the internal vector
    fn push_default(&mut self);
    /// Swap the last element in the vector with the one at the index, and then remove the last element.
    fn swap_remove(&mut self, idx: usize);
    /// Clone the values of the vector into a new one.
    fn clone_box(&self) -> Box<dyn DynDirtyVec>;
    /// Append data from another vector into this one.
    fn append_from(&mut self, other: &dyn DynDirtyVec);

    /// Convert the data store into it's Any type
    fn as_any_ref<'a>(&'a self) -> &'a dyn Any;
    /// Convert the data store into it's Any type as mutable
    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn Any;
}

impl dyn DynDirtyVec {
    /// Downcast this dynamic vector into a reference of the concrete vector type
    pub fn downcast_ref<T: 'static>(&self) -> Option<&DirtyVec<T>> {
        self.as_any_ref().downcast_ref::<DirtyVec<T>>()
    }

    /// Downcast this dynamic vector into a mutable reference of the concrete vector type
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut DirtyVec<T>> {
        self.as_any_mut().downcast_mut::<DirtyVec<T>>()
    }
}

/// A transparent wrapper for frequently updated data with dirty flag handling
#[derive(Debug)]
pub struct DataView<T> {
    data: T,
    is_dirty: Cell<bool>
}

impl<T: Clone> DataView<T> {
    pub fn new(data: T) -> Self {
        Self { data, is_dirty: Cell::new(true) }
    }

    /// Check if the data has changed
    pub fn is_dirty(&self) -> bool {
        self.is_dirty.get()
    }

    /// Mark the data as clean
    pub fn mark_clean(&self) {
        self.is_dirty.set(false);
    }

    /// Mark the data as dirty
    pub fn mark_dirty(&self) {
        self.is_dirty.set(true);
    }

    /// Set the data held in the view, marking it dirty
    pub fn set(&mut self, data: T) {
        self.data = data;
        self.is_dirty.set(true);
    }
}

impl<T: Clone> Clone for DataView<T> {
    fn clone(&self) -> Self {
        DataView::new(self.data.clone())
    }
}

impl<T> Deref for DataView<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.data }
}

impl<T> DerefMut for DataView<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_dirty.set(true);
        &mut self.data
    }
}

/// A Vec which wraps data in a DataView to track changes
pub struct DirtyVec<T> {
    pub inner: Vec<DataView<T>>
}

impl<T: Default + Clone + 'static> DynDirtyVec for DirtyVec<T> {
    fn is_dirty(&self, idx: usize) -> bool {
        self.inner.get(idx).map_or(false, |value| value.is_dirty())
    }

    fn mark_clean(&self, idx: usize) {
        if let Some(value) = self.inner.get(idx) {
            value.mark_clean();
        }
    }

    fn mark_dirty(&self, idx: usize) {
        if let Some(value) = self.inner.get(idx) {
            value.mark_dirty();
        }
    }

    fn push_default(&mut self) {
        self.inner.push(DataView::new(T::default()))
    }

    fn swap_remove(&mut self, idx: usize) {
        self.inner.swap_remove(idx);
    }

    fn clone_box(&self) -> Box<dyn DynDirtyVec> {
        Box::new(Self {
            inner: self.inner.clone()
        })
    }

    fn append_from(&mut self, other: &dyn DynDirtyVec) {
        if let Some(other_vec) = other.as_any_ref().downcast_ref::<DirtyVec<T>>() {
            for source in other_vec.inner.iter() {
                self.inner.push(source.clone());
            }
        } else {
            panic!("Expected 'other' to contain values with the same type as 'self'");
        }
    }

    fn as_any_ref<'a>(&'a self) -> &'a dyn Any { self }
    fn as_any_mut<'a>(&'a mut self) -> &'a mut dyn Any { self }
}

impl<T: Default + Clone + 'static> DirtyVec<T> {
    /// Create a dirty vector from a Vec of type T
    pub fn from_vec(data_vec: Vec<T>) -> Self {
        let dirty_vec = data_vec.into_iter()
            .map(|data| DataView::new(data))
            .collect();

        Self { inner: dirty_vec }
    }

    /// Get a reference to a raw element in the internal vector
    pub fn get_raw(&self, idx: usize) -> Option<&T> {
        Some(&*self.inner.get(idx)?)
    }

    /// Get a reference to a raw element in the internal vector
    pub fn get(&self, idx: usize) -> Option<&DataView<T>> {
        self.inner.get(idx)
    }

    /// Get a mutable reference to an element in the internal vector
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut DataView<T>> {
        self.inner.get_mut(idx)
    }
}

/// Contains helper functions to convert vertex/instance components into a byte vector and layout
pub struct PackingUtils;

impl PackingUtils {
    /// Packs all attribute data into a byte vector, filtering by components
    /// 
    /// * count: **usize** - the number of instances to pack
    /// * attributes: **&HashMap<String, AttributeData>** - the map of attribute data to filter through
    /// * components: **&Vec<Box<dyn VertexComponent>>** - the set of components to filter the attribute data with
    pub fn pack(
        count: usize, 
        vertices: &VertexData,
        components: &[Box<dyn VertexAttribute>],
    ) -> Vec<u8> {
        let mut packed = Vec::new();

        let mut sorted_components: Vec<&Box<dyn VertexAttribute>> = components.iter().collect();
        sorted_components.sort_by_key(|c| c.location());

        for i in 0..count {
            for comp in sorted_components.iter() {
                comp.write_to(i, vertices, &mut packed);
            }
        }

        packed
    }

    /// Get the updated attribute data based on the components as a vector of resource updates.
    /// 
    /// * count: **usize** - the number of instances to check for updates
    /// * attributes: **&HashMap<String, AttributeData>** - the map of attribute data to filter through
    /// * components: **&Vec<Box<dyn VertexComponent>>** - the set of components to filter the attribute data with
    pub fn get_updated(
        count: usize,
        vertices: &VertexData,
        components: &[Box<dyn VertexAttribute>],
    ) -> Vec<ResourceUpdate> {
        let stride = PackingUtils::instance_stride(components);
        let mut updates: Vec<ResourceUpdate> = Vec::new();

        let mut i = 0;
        while i < count {
            if !PackingUtils::is_instance_dirty(i, vertices, components) {
                i+=1;
                continue;
            }

            let start_idx = i; // save for the offset calculation
            let mut data_span: Vec<u8> = Vec::new();

            while i < count && PackingUtils::is_instance_dirty(i, vertices, components) {
                for comp in components {
                    comp.write_to(i, vertices, &mut data_span);
                }

                PackingUtils::mark_instance_clean(i, vertices);
                i+=1;
            }

            updates.push(ResourceUpdate { 
                data: data_span, 
                offset: (start_idx * stride) as u64
            });
        }

        updates
    }

    /// Calculate the stride of an instance based on it's components
    pub fn instance_stride(components: &[Box<dyn VertexAttribute>]) -> usize {
        let stride: u64 = components.iter()
            .map(|c| c.format().size() * c.attr_count() as u64)
            .sum();

        stride as usize
    }

    /// check if an instances attributes are dirty (changed this frame)
    fn is_instance_dirty(idx: usize, instances: &VertexData, components: &[Box<dyn VertexAttribute>]) -> bool {
        components.iter().any(|comp| {
            match instances.attributes.get(comp.name()) {
                Some(attr_vec) => attr_vec.is_dirty(idx),
                None => false
            }
        })
    }

    /// mark the attributes of an instance as clean
    fn mark_instance_clean(idx: usize, instances: &VertexData) {
        instances.attributes.iter().for_each(|(_, attr)| {
            attr.mark_clean(idx);
        });
    }

    /// creates a vertex layout builder given the step mode and vertex components
    ///
    /// * step_mode: **wgpu::VertexStepMode** - the step mode of the vertex layout
    /// * components: **&Vec<Box<dyn VertexComponent>>** - the set of components belonging to the layout
    pub fn layout_builder(
        step_mode: wgpu::VertexStepMode, 
        components: &[Box::<dyn VertexAttribute>]
    ) -> VertexLayoutBuilder {
        let mut layout = VertexLayoutBuilder::new(step_mode);
        let mut sorted_components: Vec<&Box<dyn VertexAttribute>> = components.iter().collect();
        sorted_components.sort_by_key(|c| c.location());

        for component in sorted_components {
            let mut location= component.location();
            for _ in 0..component.attr_count() {
                layout.add_attribute(location, component.format());

                location += 1;
            }
        }

        layout
    } 
}