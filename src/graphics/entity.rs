use std::sync::Arc;

use crate::graphics::{bind_group::{BindGroupLayoutBuilder, LayoutBindType, LayoutEntry, LayoutVisibility}, geometry::GeometryBuilder, material::Material, render_pipeline::RenderPipelineBuilder, transform::Transform, wpgu_context::{ResourceBinding, ResourceID, ResourceScope}};

#[derive(Clone, Debug)]
pub struct TransformGroup {
    pub layout: BindGroupLayoutBuilder,
    pub binding: ResourceBinding
}

/// Represents the state of an entity as needed by the Renderer
pub struct EntityDesciptor {
    pub geometry_key: String,
    pub namespace: String,
    pub transform_id: ResourceID,
    pub transform_group: Option<TransformGroup>,
    pub updated_buffers: Vec<(ResourceID, Vec<u8>)>,
}

#[derive(Clone, Debug)]
pub struct RenderInfo {
    pub shader_path: String,
    pub pipeline: RenderPipelineBuilder,
}

/// Simple data stuct that consolidates rendering properties for a single instance
pub struct Entity {
    pub geometry: Arc<GeometryBuilder>,
    pub transform: Transform,
    pub material: Arc<Material>,
    pub render_info: RenderInfo
}

impl Entity {
    /// Get the description of this entity
    pub fn descriptor(&self) -> EntityDesciptor {
        EntityDesciptor {
            geometry_key: self.geometry.get_label(),
            namespace: self.namespace(),
            transform_id: self.transform_id(),
            transform_group: Some(TransformGroup { 
                layout: self.transform_layout(),
                binding: self.transform_binding()
            }),
            updated_buffers: self.material.get_buffers_updated()
        }
    }

    fn transform_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::transform_{}", self.geometry.get_label(), self.transform.id()),
            scope: ResourceScope::Entity
        }
    }

    fn transform_layout(&self) -> BindGroupLayoutBuilder {
        BindGroupLayoutBuilder::new()
            .with_label("transform")
            .with_entry(LayoutEntry {
                binding: 0,
                visibility: LayoutVisibility::Vertex,
                ty: LayoutBindType::Uniform
            })
    }

    fn transform_binding(&self) -> ResourceBinding {
        ResourceBinding { 
            id: self.transform_id(), 
            slot: 0 
        }
    }

    fn namespace(&self) -> String {
        format!("{}::{}", self.geometry.get_label(), self.material.get_key())
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
    pub material: Arc<Material>,
    pub render_info: RenderInfo,
    pub transforms: TransformInstances,
}

impl EntityInstances {
    /// Get the description of this entity
    pub fn descriptor(&self) -> EntityDesciptor {
        EntityDesciptor {
            geometry_key: self.geometry.get_label(),
            namespace: self.namespace(),
            transform_id: self.transform_id(),
            transform_group: None,
            updated_buffers: self.material.get_buffers_updated()
        }
    }

    // get the entity-material namespace id for this entity
    fn namespace(&self) -> String {
        format!("{}::{}", self.geometry.get_label(), self.material.get_key())
    }

    fn transform_id(&self) -> ResourceID {
        ResourceID {
            key: format!("{}::transforms_{}", self.geometry.get_label(), self.transforms.key),
            scope: ResourceScope::Entity
        }
    }
}