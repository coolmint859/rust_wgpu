use std::sync::atomic::{AtomicU32, Ordering};

use crate::graphics::{geometry::Geometry, instance::{Instance, InstanceGroup, InstanceMut, InstanceTemplate}, material::{Material, UniformBuilder}, render_pipeline::RenderPipelineBuilder, transform::Transform, vertex::TransformAttribute, wpgu_context::{GeometryID, ResourceBinding, ResourceID, ResourceScope, ResourceType, ResourceUpdate}};

static ENTITY_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug)]
pub struct RenderInfo {
    pub shader_path: String,
    pub pipeline: RenderPipelineBuilder,
}

/// A high level struct representing instances of renderable geometry
pub struct Primitive {
    id: u32,
    pub label: String,
    pub geometry: Geometry,
    pub instances: InstanceGroup,
    pub material: Material,
    pub render_info: RenderInfo
}

impl Primitive {
    /// Create a primitive with a single instance
    pub fn new(label: &str, geometry: Geometry, material: Material, transform: Transform, render_info: RenderInfo) -> Self {
        let instances = InstanceGroup::new(1, 1)
            .with_label(label)
            .with_attribute(TransformAttribute, vec![transform]);
        Primitive::from_group(label, geometry, material, instances, render_info)
    }

    /// Create a primitive with multiple instances (just transforms)
    pub fn new_instanced(label: &str, geometry: Geometry, material: Material, instances: Vec<Transform>, render_info: RenderInfo) -> Self {
        let instances = InstanceGroup::new(instances.len(), instances.capacity())
            .with_label(label)
            .with_attribute(TransformAttribute, instances);
        
        Primitive::from_group(label, geometry, material, instances, render_info)
    }

    /// Create a primitive with a custom instance group
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

    /// Get an empty template of this primitive's instances.
    /// 
    /// This is useful when adding new instances to the primitive as it is.
    pub fn get_template(&self) -> InstanceTemplate {
        let mut template = InstanceTemplate::new();

        for (attr, dyn_vec) in self.instances.get_instances().properties.iter() {
            template.set_attribute_vec(&attr, dyn_vec.clone_empty());
        }

        template
    }

    /// Get a reference to the first instance of the primitive, if exists
    ///
    /// Useful in cases where a primitive only has one instance
    pub fn get_first(&self) -> Option<Instance<'_>> {
        self.instances.get_instance(0)
    }

    /// Get a reference to the first instance of the primitive
    ///
    /// Useful in cases where a primitive only has one instance
    /// 
    /// ## Panics
    /// If the primitive has no instances defined
    pub fn first(&self) -> Instance<'_> {
        self.instances.get_instance(0).expect("Expected this primitive to have at least one isntance, but none where found.")
    }

    /// Get a mutable reference to the first instance of the primitive, if exists.
    /// 
    /// Useful in cases where an primitive only has one instance
    pub fn get_first_mut(&mut self) -> Option<InstanceMut<'_>> {
        self.instances.get_instance_mut(0)
    }

    /// Get a mutable reference to the first instance of the primitive
    ///
    /// Useful in cases where an primitive only has one instance
    /// 
    /// ## Panics
    /// If the primitive has no instances defined
    pub fn first_mut(&mut self) -> InstanceMut<'_> {
        self.instances.get_instance_mut(0).expect("Expected this primitive to have at least one isntance, but none where found.")
    }

    /// Get the resource id for the instance buffer associated with this primitive.
    pub fn instance_id(&self) -> ResourceID {
        ResourceID {
            key: self.primitive_namespace(&self.instances.get_label()),
            scope: ResourceScope::Primitive,
            r_type: ResourceType::Instance(self.instances.get_layout_builder())
        }
    }

    /// Get the resource id for the geometry buffers associated with this primitive.
    pub fn geometry_id(&self) -> GeometryID {
        self.geometry.get_ids()
    }

    /// Get the resource id for this primitive's material bind group.
    pub fn material_id(&self) -> ResourceID {
        ResourceID {
            key: self.primitive_namespace(&self.geometry.get_label()),
            scope: ResourceScope::Primitive,
            r_type: ResourceType::BindGroup
        }
    }

    /// Get the uniforms associated with this primitive's material, namespaced to the primitive.
    pub fn get_uniforms(&self) -> Vec<(ResourceBinding, UniformBuilder)> {
        let mut uniforms = self.material.get_uniforms();
        for (binding, _) in &mut uniforms {
            match binding.id.scope {
                ResourceScope::Primitive => binding.id.key = self.primitive_namespace(&binding.id.key),
                _ => ()
            };
        }

        uniforms
    }

    /// Get this primitive's uniform data as a series of updates
    pub fn uniform_updates(&mut self) -> Vec<(ResourceID, ResourceUpdate)> {
        let mut updated = self.material.get_updated();
        for (id, _) in &mut updated {
            match id.scope {
                ResourceScope::Primitive => id.key = self.primitive_namespace(&id.key),
                _ => ()
            };
        }

        updated
    }

    /// Namespace the provided resource key to this primitive
    fn primitive_namespace(&self, resource_key: &String) -> String {
        format!("{}_{}::{}", self.geometry.get_label(), self.id, resource_key)
    }
}