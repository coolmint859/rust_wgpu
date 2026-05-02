use std::sync::Arc;

use crate::graphics::{bind_group::{BindGroupLayoutBuilder, LayoutBindType, LayoutEntry, LayoutVisibility}, geometry::GeometryBuilder, material::{Material, UniformBuilder}, render_pipeline::RenderPipelineBuilder, transform::Transform, wpgu_context::{ResourceBinding, ResourceID, ResourceScope}};


#[derive(Clone, Debug)]
pub struct RenderInfo {
    pub shader_path: String,
    pub pipeline: RenderPipelineBuilder,
}

/// Simple data stuct that consolidates rendering properties for a single instance
pub struct Entity {
    pub geometry: Arc<GeometryBuilder>,
    pub transform: Transform,
    pub material: Material,
    pub render_info: RenderInfo
}

impl Entity {
    /// Get the id of this entity's transform
    pub fn transform_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::transform_{}", self.geometry.get_label(), self.transform.id()),
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

    /// Get the layout for this entity's transform
    pub fn transform_layout(&self) -> BindGroupLayoutBuilder {
        BindGroupLayoutBuilder::new()
            .with_label("transform")
            .with_entry(LayoutEntry {
                binding: 0,
                visibility: LayoutVisibility::Vertex,
                ty: LayoutBindType::Uniform
            })
    }

    /// Get the binding for this entity's transform
    pub fn transform_binding(&self) -> ResourceBinding {
        ResourceBinding { 
            id: self.transform_id(), 
            slot: 0 
        }
    }

    /// Get the uniforms associated with this entity's material namespaced to the entity.
    pub fn get_uniforms(&self) -> Vec<(ResourceBinding, UniformBuilder)> {
        let mut uniforms = self.material.get_uniforms();
        for (binding, _) in &mut uniforms {
            self.uniform_namespace(&mut binding.id);
        }

        uniforms
    }

    pub fn get_updated(&mut self) -> Vec<(ResourceID, Vec<u8>)> {
        let mut updated = self.material.get_buffers_updated();
        for (id, _) in &mut updated {
            self.uniform_namespace(id);
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

/// Allows transform data for instanced entities to be uniquely keyed
pub struct TransformInstances {
    pub key: String, 
    pub data: Vec<Transform>
}

/// Data stuct that consolidates rendering properties for multiple instances
pub struct EntityInstances {
    pub geometry: Arc<GeometryBuilder>,
    pub material: Material,
    pub render_info: RenderInfo,
    pub transforms: TransformInstances,
}

impl EntityInstances {
    pub fn transform_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::transforms_{}", self.geometry.get_label(), self.transforms.key),
            scope: ResourceScope::Entity
        }
    }

    /// Get the uniforms associated with this entity's material namespaced to the entity.
    pub fn get_uniforms(&self) -> Vec<(ResourceBinding, UniformBuilder)> {
        let mut uniforms = self.material.get_uniforms();
        for (binding, _) in &mut uniforms {
            self.uniform_namespace(&mut binding.id);
        }

        uniforms
    }

    pub fn get_updated(&mut self) -> Vec<(ResourceID, Vec<u8>)> {
        let mut updated = self.material.get_buffers_updated();
        for (id, _) in &mut updated {
            self.uniform_namespace(id);
        }

        updated
    }

    // get the entity-material namespace id for this entity
    pub fn material_key(&self) -> String {
        format!("{}::{}", self.geometry.get_label(), self.material.get_key())
    }

    /// Get the layout builder for this entity's material
    pub fn material_layout(&self) -> BindGroupLayoutBuilder {
        self.material.get_layout_builder()
    }

    /// get the namespace of a uniform
    fn uniform_namespace(&self, id: &mut ResourceID) {
        match id.scope {
            ResourceScope::Entity => id.key = format!("{}::{}", self.geometry.get_label(), id.key),
            _ => ()
        };
    }
}