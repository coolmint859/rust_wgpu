use std::sync::atomic::{AtomicU32, Ordering};

use crate::graphics::{geometry::Geometry, instance::{Instance, InstanceGroup, InstanceMut, TintAttribute, TransformAttribute}, material::{Material, UniformBuilder}, render_pipeline::RenderPipelineBuilder, transform::Transform, wpgu_context::{GeometryID, ResourceBinding, ResourceID, ResourceScope, ResourceType, ResourceUpdate}};

static ENTITY_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug)]
pub struct RenderInfo {
    pub shader_path: String,
    pub pipeline: RenderPipelineBuilder,
}

/// Consolidates render info for multiple instances of an entity
pub struct Entity {
    id: u32,
    pub label: String,
    pub geometry: Geometry,
    pub instances: InstanceGroup,
    pub material: Material,
    pub render_info: RenderInfo
}

impl Entity {
    /// Create an entity with a single instance
    pub fn new(label: &str, geometry: Geometry, material: Material, transform: Transform, render_info: RenderInfo) -> Self {
        let instances = InstanceGroup::new(1, 1)
            .with_label(label)
            .with_attribute(TransformAttribute, vec![transform])
            .with_attribute(TintAttribute, vec![glam::Vec4::ONE]);

        Entity::from_group(label, geometry, material, instances, render_info)
    }

    /// Create an entity with multiple instances (just transforms)
    pub fn new_instanced(label: &str, geometry: Geometry, material: Material, instances: Vec<Transform>, render_info: RenderInfo) -> Self {
        let instances = InstanceGroup::new(instances.len(), instances.capacity())
            .with_label(label)
            .with_attribute(TransformAttribute, instances);
        
        Entity::from_group(label, geometry, material, instances, render_info)
    }

    /// Create an entity with a custom instance group
    pub fn from_group(label: &str, geometry: Geometry, material: Material, instances: InstanceGroup, render_info: RenderInfo) -> Self {
        let id = ENTITY_COUNTER.fetch_add(1, Ordering::SeqCst);
        
        Self { 
            id, 
            label: label.to_string(),
            geometry, 
            material, 
            instances,
            render_info 
        }
    }

    /// Get a reference to the first instance of the entity
    ///
    /// Useful in cases where an entity only has one instance
    pub fn get_first(&self) -> Option<Instance<'_>> {
        self.instances.get_instance(0)
    }

    /// Get a reference to the first instance of the entity
    ///
    /// Useful in cases where an entity only has one instance
    /// 
    /// ## Panics
    /// If the entity has no instances defined
    pub fn first(&self) -> Instance<'_> {
        self.instances.get_instance(0).expect("Expected this entity to have at least one isntance, but none where found.")
    }

    /// Get a mutable reference to the first instance of the entity.
    /// 
    /// Useful in cases where an entity only has one instance
    pub fn get_first_mut(&mut self) -> InstanceMut<'_> {
        self.instances.get_instance_mut(0).unwrap()
    }

    /// Get a mutable reference to the first instance of the entity
    ///
    /// Useful in cases where an entity only has one instance
    /// 
    /// ## Panics
    /// If the entity has no instances defined
    pub fn first_mut(&mut self) -> InstanceMut<'_> {
        self.instances.get_instance_mut(0).expect("Expected this entity to have at least one isntance, but none where found.")
    }

    /// Get the resource id for the instance buffer associated with this entity.
    pub fn instance_id(&self) -> ResourceID {
        ResourceID {
            key: self.entity_namespace(&self.instances.get_label()),
            scope: ResourceScope::Entity,
            r_type: ResourceType::Instance(self.instances.get_layout_builder())
        }
    }

    /// Get the resource id for the geometry buffers associated with this entity.
    pub fn geometry_id(&self) -> GeometryID {
        self.geometry.get_ids()
    }

    /// Get the resource id for this entity's material bind group.
    pub fn material_id(&self) -> ResourceID {
        ResourceID {
            key: self.entity_namespace(&self.geometry.get_label()),
            scope: ResourceScope::Entity,
            r_type: ResourceType::BindGroup
        }
    }

    /// Get the uniforms associated with this entity's material namespaced to the entity.
    pub fn get_uniforms(&self) -> Vec<(ResourceBinding, UniformBuilder)> {
        let mut uniforms = self.material.get_uniforms();
        for (binding, _) in &mut uniforms {
            match binding.id.scope {
                ResourceScope::Entity => binding.id.key = self.entity_namespace(&binding.id.key),
                _ => ()
            };
        }

        uniforms
    }

    /// Get this entity's uniform data as a series of updates
    pub fn uniform_updates(&mut self) -> Vec<(ResourceID, ResourceUpdate)> {
        let mut updated = self.material.get_updated();
        for (id, _) in &mut updated {
            match id.scope {
                ResourceScope::Entity => id.key = self.entity_namespace(&id.key),
                _ => ()
            };
        }

        updated
    }

    /// Namespace the provided resource key to this entity
    fn entity_namespace(&self, resource_key: &String) -> String {
        format!("{}_{}::{}", self.geometry.get_label(), self.id, resource_key)
    }
}