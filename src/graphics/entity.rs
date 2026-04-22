use std::sync::Arc;

use crate::graphics::{bind_group::{BindGroupLayoutBuilder, LayoutBindType, LayoutEntry, LayoutVisibility}, material::Material, mesh::Mesh, render_pipeline::RenderPipelineBuilder, transform::Transform, wpgu_context::{ResourceBinding, ResourceID, ResourceScope}};

/// Simple data stuct used to consolidate rendering properties
pub struct Entity {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material: Arc<Material>,
    pub pipeline: RenderPipelineBuilder
}

impl Entity {
    /// get the unique id for this entity
    pub fn id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::uniforms", self.mesh.get_key()),
            scope: ResourceScope::Entity
        }
    }

    /// Get the resource binding for this entity's transform
    pub fn transform_binding(&self) -> ResourceBinding {
        ResourceBinding {
            id: self.id(),
            slot: 0
        }
    }

    /// Get this entity's transform's layout entry
    pub fn transform_layout(&self) -> BindGroupLayoutBuilder {
        BindGroupLayoutBuilder::new()
            .with_label("entity-transform")
            .with_entry(LayoutEntry { 
                binding: 0, 
                visibility: LayoutVisibility::Vertex, 
                ty: LayoutBindType::Uniform
            })
    }

    /// get the entity-material namespace id for this entity
    pub fn get_namespace_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::{}", self.mesh.get_key(), self.material.get_key()),
            scope: ResourceScope::Entity
        }
    }
}