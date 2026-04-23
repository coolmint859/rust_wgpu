use std::sync::Arc;

use crate::graphics::{bind_group::{BindGroupLayoutBuilder, LayoutBindType, LayoutEntry, LayoutVisibility}, material::Material, mesh::Mesh, render_pipeline::RenderPipelineBuilder, transform::Transform, wpgu_context::{ResourceBinding, ResourceID, ResourceScope}};

pub trait EntityTrait {
    /// get the transform id for this entity
    fn transform_id(&self) -> ResourceID;

    // get the entity-material namespace id for this entity
    fn namespace_id(&self) -> ResourceID;
}

/// Simple data stuct that consolidates rendering properties for a single instance
pub struct Entity {
    pub mesh: Mesh,
    pub transform: Transform,
    pub material: Arc<Material>,
    pub pipeline: RenderPipelineBuilder
}

impl Entity {
    /// Get the resource binding for this entity's transform
    pub fn transform_binding(&self) -> ResourceBinding {
        ResourceBinding {
            id: self.transform_id(),
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
}

impl EntityTrait for Entity {
    fn transform_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::transform", self.mesh.get_key()),
            scope: ResourceScope::Entity
        }
    }

    // get the entity-material namespace id for this entity
    fn namespace_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::{}", self.mesh.get_key(), self.material.get_key()),
            scope: ResourceScope::Entity
        }
    }
}

/// Data stuct that consolidates rendering properties for multiple instances
pub struct EntityInstances {
    pub mesh: Mesh,
    pub material: Arc<Material>,
    pub pipeline: RenderPipelineBuilder,
    pub transforms: Vec<Transform>,
}

impl EntityTrait for EntityInstances {
    fn transform_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::instance_transforms", self.mesh.get_key()),
            scope: ResourceScope::Entity
        }
    }

    // get the entity-material namespace id for this entity
    fn namespace_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::{}", self.mesh.get_key(), self.material.get_key()),
            scope: ResourceScope::Entity
        }
    }
}