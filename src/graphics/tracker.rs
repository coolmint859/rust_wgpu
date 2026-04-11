use std::collections::HashSet;

use crate::graphics::{bind_group::ResourceID, render_pipeline::RenderPipelineTemplate};

/// Tracks resource requests, allowing the transient Renderer to optimize it's command generation
#[derive(Clone, Debug)]
pub struct ResourceTracker {
    pub bg_layouts: HashSet<String>,
    pub bind_groups: HashSet<String>,
    pub buffers: HashSet<ResourceID>,
    pub pipelines: HashSet<RenderPipelineTemplate>,
    pub meshes: HashSet<u32>
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self {
            bg_layouts: HashSet::new(),
            bind_groups: HashSet::new(),
            buffers: HashSet::new(),
            pipelines: HashSet::new(),
            meshes: HashSet::new(),
        }
    }

    /// Reset the state of this tracker
    pub fn clear(&mut self) {
        self.bg_layouts.clear();
        self.bind_groups.clear();
        self.buffers.clear();
        self.meshes.clear();
        self.pipelines.clear();
    }

    /// Copy the resources in another tracker into this one
    pub fn copy_from(&mut self, other: &ResourceTracker) {
        for layout in &other.bg_layouts {
            self.bg_layouts.insert(layout.clone());
        }
        for bind_group in &other.bind_groups {
            self.bind_groups.insert(bind_group.clone());
        }
        for mesh in &other.meshes {
            self.meshes.insert(mesh.clone());
        }
        for pipeline in &other.pipelines {
            self.pipelines.insert(pipeline.clone());
        }
        for buffer in &other.buffers {
            self.buffers.insert(buffer.clone());
        }
    }
}
