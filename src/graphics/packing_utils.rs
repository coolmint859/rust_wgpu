#![allow(dead_code)]
use glam::*;

use crate::graphics::{data_table::DataTable, vertex::{VertexAttribute, VertexLayoutBuilder}, wpgu_context::ResourceUpdate};

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
        vertices: &DataTable,
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
        vertices: &DataTable,
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
    fn is_instance_dirty(idx: usize, instances: &DataTable, components: &[Box<dyn VertexAttribute>]) -> bool {
        components.iter().any(|comp| {
            instances.properties.get(comp.name())
                .map_or(false, |attr| {
                    if let Some(dirty_tracker) = attr.as_dirty_tracker() {
                        return dirty_tracker.is_dirty(idx)
                    }
                    return false;
                })
        })
    }

    /// mark the attributes of an instance as clean
    fn mark_instance_clean(idx: usize, instances: &DataTable) {
        instances.properties.iter().for_each(|(_, attr)| {
            if let Some(attr_dirty) = attr.as_dirty_tracker() {
                attr_dirty.mark_clean(idx);
            }
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