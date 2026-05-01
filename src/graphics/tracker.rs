#![allow(dead_code)]
use std::collections::HashSet;

use crate::graphics::{render_pipeline::RenderPipelineBuilder, wpgu_context::ResourceID};

/// Tracks resource requests, allowing the transient Renderer to optimize it's command generation
#[derive(Clone, Debug)]
pub struct ResourceTracker {
    pub shader_specs: HashSet<String>,
    pub shader_mods: HashSet<String>,
    pub pipelines: HashSet<RenderPipelineBuilder>,

    pub bind_groups: HashSet<String>,
    pub geometries: HashSet<String>,
    
    pub buffers: HashSet<ResourceID>,
    pub textures: HashSet<ResourceID>,
    pub samplers: HashSet<ResourceID>,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            shader_specs: HashSet::new(),
            shader_mods: HashSet::new(),
            bind_groups: HashSet::new(),
            buffers: HashSet::new(),
            pipelines: HashSet::new(),
            geometries: HashSet::new(),
            textures: HashSet::new(),
            samplers: HashSet::new(),
        }
    }

    /// Reset the state of this tracker
    pub fn clear(&mut self) {
        self.shader_specs.clear();
        self.shader_mods.clear();
        self.bind_groups.clear();
        self.buffers.clear();
        self.geometries.clear();
        self.pipelines.clear();
        self.textures.clear();
        self.samplers.clear();
    }

    /// Copy the resources in another tracker into this one
    pub fn copy_from(&mut self, other: &ResourceTracker) {
        for shader_spec in &other.shader_specs {
            self.shader_specs.insert(shader_spec.clone());
        }
        for shader_mod in &other.shader_mods {
            self.shader_mods.insert(shader_mod.clone());
        }
        for bind_group in &other.bind_groups {
            self.bind_groups.insert(bind_group.clone());
        }
        for mesh in &other.geometries {
            self.geometries.insert(mesh.clone());
        }
        for pipeline in &other.pipelines {
            self.pipelines.insert(pipeline.clone());
        }
        for buffer in &other.buffers {
            self.buffers.insert(buffer.clone());
        }
        for texture in &other.textures {
            self.textures.insert(texture.clone());
        }
        for sampler in &other.samplers {
            self.samplers.insert(sampler.clone());
        }
    }
}
