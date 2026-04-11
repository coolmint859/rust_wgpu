#![allow(dead_code)]
use std::{hash::Hash, sync::Arc};

use super::gpu_resource::ResourceBuilder;

/// A composite key for resources stored in a bind group
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct ResourceID {
    pub group_id: String,
    pub binding: u32,
}

/// Represents a single bind group layout entry
#[derive(Clone, Debug)]
pub struct LayoutEntry {
    pub binding: u32,
    pub visibility: wgpu::ShaderStages,
    pub ty: LayoutBindType,
}

/// The shader stages the bind group is visible to
pub enum LayoutVisibility {
    /// Bind group is visible to the vertex stage
    Vertex,
    /// Bind group is visible to the fragment stage
    Fragment,
    /// Bind group is visible to the vertex and fragment stages
    VertexFragment,
}

/// Bind Group Entry layout types
#[derive(Clone, Debug)]
pub enum LayoutBindType {
    Uniform,
    Storage(bool),
    Texture,
    Sampler,
}

#[derive(Clone, Debug)]
pub struct BindGroupLayoutBuilder {
    pub label: String,
    pub entries: Vec<LayoutEntry>
}

impl BindGroupLayoutBuilder {
    pub fn new() -> Self {
        Self {
            label: "bgl-builder".to_string(),
            entries: Vec::new() 
        }
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add an entry into the Bind Group Layout.
    /// 
    /// Note: bind slots are determined by order - the first entry added has bind slot 0, second has slot 1, etc..
    pub fn with_entry(mut self, visibility: LayoutVisibility, bind_type: LayoutBindType) -> Self {
        let bind_vis = match visibility {
            LayoutVisibility::Vertex => wgpu::ShaderStages::VERTEX,
            LayoutVisibility::Fragment => wgpu::ShaderStages::FRAGMENT,
            LayoutVisibility::VertexFragment => wgpu::ShaderStages::VERTEX_FRAGMENT
        };

        let bind_slot = self.entries.len() as u32;
        self.entries.push(LayoutEntry { 
            binding: bind_slot, 
            visibility: bind_vis, 
            ty: bind_type
        });

        self
    }

    /// Add a uniform buffer entry into the Bind Group Layout.
    /// 
    /// Note: bind slots are determined by order - the first entry added has bind slot 0, second has slot 1, etc..
    pub fn with_uniform_entry(self, visibility: LayoutVisibility) -> Self {
        self.with_entry(visibility, LayoutBindType::Uniform)
    }

    /// Add a texture entry into the Bind Group Layout.
    /// 
    /// Note: bind slots are determined by order - the first entry added has bind slot 0, second has slot 1, etc..
    pub fn with_texture_entry(self, visibility: LayoutVisibility) -> Self {
        self.with_entry(visibility, LayoutBindType::Texture)
    }

    /// helper function to create a binding type
    fn type_descriptor(ty: &LayoutBindType) -> wgpu::BindingType {
        match ty {
            LayoutBindType::Uniform => wgpu::BindingType::Buffer { 
                ty: wgpu::BufferBindingType::Uniform, 
                has_dynamic_offset: false, 
                min_binding_size: None
            },
            LayoutBindType::Storage(read_only) => wgpu::BindingType::Buffer { 
                ty: wgpu::BufferBindingType::Storage { read_only: *read_only }, 
                has_dynamic_offset: false, 
                min_binding_size: None 
            },
            LayoutBindType::Texture => wgpu::BindingType::Texture { 
                sample_type: wgpu::TextureSampleType::Float { filterable: false }, 
                view_dimension: wgpu::TextureViewDimension::D2, 
                multisampled: false,
            },
            LayoutBindType::Sampler => wgpu::BindingType::Sampler( 
                wgpu::SamplerBindingType::NonFiltering
            )
        }
    }
}

impl ResourceBuilder for BindGroupLayoutBuilder {
    type Output = Arc<wgpu::BindGroupLayout>;
    type Context = wgpu::Device;

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        let mut group_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();

        for entry in &self.entries {
            group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: entry.binding,
                visibility: entry.visibility,
                ty: BindGroupLayoutBuilder::type_descriptor(&entry.ty),
                count: None
            });
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(format!("[Bind Group Layout] @{}", &self.label).as_str()),
            entries: &group_entries,
        });

        println!("[Bind Group Layout] Created new bind group layout with label '{}'", self.label);

        Ok(Arc::new(layout))
    }
}

/// A specific resource stored in a bind group entry
#[derive(Clone, Debug)]
pub enum BindGroupResource {
    Buffer(Arc<wgpu::Buffer>),
    Texture(Arc<wgpu::TextureView>),
    Sampler(Arc<wgpu::Sampler>)
}

/// Represents a single uniform on the gpu
#[derive(Clone, Debug)]
pub struct BindGroupEntry {
    pub bind_slot: u32,
    pub resource: BindGroupResource,
}

/// Represents a group of uniform data to be send to the gpu
#[derive(Clone, Debug)]
pub struct BindGroupBuilder {
    pub label: String,
    pub entries: Vec<BindGroupEntry>,
    pub layout: Arc<wgpu::BindGroupLayout>,
}

impl BindGroupBuilder {
    pub fn new(layout: Arc<wgpu::BindGroupLayout>) -> Self {
        Self {
            label: "bind-group-builder".to_string(),
            entries: Vec::new(),
            layout: Arc::clone(&layout)
        }
    }

    /// Add a list of resources to this bind group
    pub fn with_resources(mut self, resources: Vec<(ResourceID, BindGroupResource)>) -> Self {
        for (id, resource) in resources {
            self.entries.push(BindGroupEntry { 
                bind_slot: id.binding, 
                resource 
            });
        }
        self
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }
}

impl ResourceBuilder for BindGroupBuilder {
    type Output = wgpu::BindGroup;
    type Context = wgpu::Device;

    fn build(&self, device: Arc<wgpu::Device>) -> Result<wgpu::BindGroup, String> {
        let group_entries: Vec<wgpu::BindGroupEntry> = self.entries.iter()
            .map(|entry| {
                let bind_resource = match &entry.resource {
                    BindGroupResource::Buffer(buffer) => {
                        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer,
                            offset: 0,
                            size: None
                        })
                    },
                    BindGroupResource::Texture(view) => {
                        wgpu::BindingResource::TextureView(&view)
                    },
                    BindGroupResource::Sampler(sampler) => {
                        wgpu::BindingResource::Sampler(&sampler)
                    }
                };

                wgpu::BindGroupEntry {
                    binding: entry.bind_slot,
                    resource: bind_resource
                }
            })
            .collect();

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("Bind Group for '{}'", self.label)),
                layout: &self.layout,
                entries: &group_entries
            }
        );

        println!("[Bind Group] Created new bind group with label '{}'", self.label);

        Ok(bind_group)
    }
}