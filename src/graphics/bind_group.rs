#![allow(dead_code)]
use std::{sync::Arc};

use super::handler::ResourceBuilder;

/// Represents a single bind group layout entry
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct LayoutEntry {
    pub binding: u32,
    pub visibility: LayoutVisibility,
    pub ty: LayoutBindType,
}

/// Represents a single bind group layout entry
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
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
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum LayoutBindType {
    /// uniform buffers
    Uniform,
    /// storage buffers
    Storage(bool),
    /// textures and texture views
    Texture,
    /// texture samplers
    Sampler,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct BindGroupLayoutBuilder {
    pub label: String,
    pub entries: Vec<LayoutEntry>,
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
    pub fn with_entry(mut self, entry: LayoutEntry) -> Self {
        self.entries.push(entry);
        self
    }

    /// Add an entry into the Bind Group Layout.
    pub fn add_entry(&mut self, entry: LayoutEntry) {
        self.entries.push(entry);
    }

    /// Check if an entry with the provided bind slot has previously been added
    pub fn has_binding(&self, bind_slot: u32) -> bool {
        for entry in &self.entries {
            if entry.binding == bind_slot { return true }
        }
        return false;
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
            let bind_vis = match &entry.visibility {
                LayoutVisibility::Vertex => wgpu::ShaderStages::VERTEX,
                LayoutVisibility::Fragment => wgpu::ShaderStages::FRAGMENT,
                LayoutVisibility::VertexFragment => wgpu::ShaderStages::VERTEX_FRAGMENT
            };

            group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: entry.binding,
                visibility: bind_vis,
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

/// Houses the environment needed to construct bind groups
#[derive(Clone)]
pub struct BindGroupContext {
    pub device: Arc<wgpu::Device>,
    pub layout: Arc<wgpu::BindGroupLayout>
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
}

impl BindGroupBuilder {
    pub fn new() -> Self {
        Self {
            label: "bind-group-builder".to_string(),
            entries: Vec::new(),
        }
    }

    /// Add a list of resources to this bind group
    pub fn with_resources(mut self, resources: Vec<(u32, BindGroupResource)>) -> Self {
        for (bind_slot, resource) in resources {
            self.entries.push(BindGroupEntry { 
                bind_slot, 
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
    type Context = BindGroupContext;

    fn build(&self, context: Arc<BindGroupContext>) -> Result<wgpu::BindGroup, String> {
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

        let bind_group = context.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some(&format!("Bind Group for '{}'", self.label)),
                layout: &context.layout,
                entries: &group_entries
            }
        );

        println!("[Bind Group] Created new bind group with label '{}'", self.label);

        Ok(bind_group)
    }
}