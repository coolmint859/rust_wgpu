use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

use crate::graphics::{bind_group::BindGroupLayoutBuilder, geometry::GeometryBuilder, material::{Material, UniformBuilder}, render_pipeline::RenderPipelineBuilder, transform::Transform, wpgu_context::{ResourceUpdate, ResourceBinding, ResourceID, ResourceScope}};

static ENTITY_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug)]
pub struct RenderInfo {
    pub shader_path: String,
    pub pipeline: RenderPipelineBuilder,
}

/// Consolidates render info for multiple instances of an entity
pub struct Entity {
    id: u32,
    label: String,
    pub geometry: Arc<GeometryBuilder>,
    pub transforms: Vec<Transform>,
    pub material: Material,
    pub render_info: RenderInfo
}

impl Entity {
    pub fn new(label: &str, geometry: Arc<GeometryBuilder>, material: Material, transform: Transform, render_info: RenderInfo) -> Self {
        let id = ENTITY_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self { 
            id, 
            label: label.to_string(),
            geometry, 
            material, 
            transforms: vec![transform], 
            render_info 
        }
    }

    /// Create an entity with multiple instances
    pub fn new_instanced(label: &str, geometry: Arc<GeometryBuilder>, material: Material, transforms: Vec<Transform>, render_info: RenderInfo) -> Self {
        let id = ENTITY_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self { 
            id, 
            label: label.to_string(),
            geometry, 
            material, 
            transforms,
            render_info 
        }
    }

    /// Get this entity's transforms as series of buffer updates
    /// 
    /// * **is_init:** *bool* - if true, all instance transforms will be packed into one update, 
    /// otherwise, only contiguous transforms that have changed will be packed.
    pub fn transform_updates(&mut self, is_init: bool) -> Vec<ResourceUpdate> {
        let mut updates = Vec::new();

        if is_init {
            updates.push(self.create_transform_update(0, self.transforms.len()));
            return updates;
        }

        let mut span_start: Option<usize> = None;
        for i in 0..self.transforms.len() {
            let is_dirty = self.transforms[i].to_updated();

            if is_dirty && span_start.is_none() {
                span_start = Some(i);
            } else if !is_dirty && span_start.is_some() {
                let start = span_start.unwrap();
                updates.push(self.create_transform_update(start, i));
                span_start = None;
            }
        }

        if let Some(start) = span_start {
            updates.push(self.create_transform_update(start, self.transforms.len()))
        }

        // println!("{:?}", updates);
        updates
    }

    /// create a span of packed transform data
    fn create_transform_update(&self, start_idx: usize, end_idx: usize) -> ResourceUpdate {
        let mut data = Vec::new();
        for i in start_idx..end_idx {
            let matrix = self.transforms[i].world_matrix();
            data.extend_from_slice(bytemuck::bytes_of(&matrix));
        }

        ResourceUpdate { 
            id: self.transform_id(), 
            data, 
            offset: (start_idx * Transform::size()) as u64 
        }
    }

    /// Get the id of this entity's transform(s)
    pub fn transform_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::{}_{}::transforms", self.label, self.geometry.get_label(), self.id),
            scope: ResourceScope::Entity
        }
    }

    /// Get the key to the bind group of this entity's material
    pub fn material_key(&self) -> String {
        format!("{}::{}", self.geometry.get_label(), self.material.get_key())
    }

    /// Get the layout builder for this entity's material
    pub fn material_layout(&self) -> BindGroupLayoutBuilder {
        self.material.get_layout_builder()
    }

    /// Get the uniforms associated with this entity's material namespaced to the entity.
    pub fn get_uniforms(&self) -> Vec<(ResourceBinding, UniformBuilder)> {
        let mut uniforms = self.material.get_uniforms();
        for (binding, _) in &mut uniforms {
            self.uniform_namespace(&mut binding.id);
        }

        uniforms
    }

    /// Get this entity's uniform data as a series of updates
    pub fn uniform_updates(&mut self) -> Vec<ResourceUpdate> {
        let mut updated = self.material.get_updated();
        for update in &mut updated {
            self.uniform_namespace(&mut update.id);
        }

        updated
    }

    /// get the namespace of a uniform
    fn uniform_namespace(&self, id: &mut ResourceID) {
        match id.scope {
            ResourceScope::Entity => id.key = format!("{}::{}", self.geometry.get_label(), id.key),
            _ => ()
        };
    }
}