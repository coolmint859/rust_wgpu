use std::sync::Arc;

use super::gpu_resource::ResourceBuilder;

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
    Storage,
    Texture,
    Sampler,
}

#[derive(Clone, Debug)]
pub struct BindGroupLayoutBuilder {
    pub key: String,
    // pub group_id: u32,
    pub entries: Vec<LayoutEntry>
}

impl BindGroupLayoutBuilder {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            // group_id: 0,
            entries: Vec::new()
        }
    }

    // /// Set the group id the bind group should be identified with
    // pub fn with_group_id(mut self, id: u32) -> Self {
    //     self.group_id = id;
    //     self
    // }

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
}

impl ResourceBuilder for BindGroupLayoutBuilder {
    type Key = String;
    type Output = Arc<wgpu::BindGroupLayout>;

    fn get_key(&self) -> Self::Key {
        self.key.clone()
    }

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        let mut group_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();

        for entry in &self.entries {
            group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: entry.binding,
                visibility: entry.visibility,
                ty: match entry.ty {
                    LayoutBindType::Uniform => wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Uniform, 
                        has_dynamic_offset: false, 
                        min_binding_size: None
                    },
                    LayoutBindType::Storage => wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Storage { read_only: false }, 
                        has_dynamic_offset: false, 
                        min_binding_size: None 
                    },
                    _ => unimplemented!("More types coming soon!")
                },
                count: None
            });
        }

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(format!("[Bind Group Layout] @{}", &self.key).as_str()),
            entries: &group_entries,
        });

        println!("Created bind group layout '{}'", self.key);

        Ok(Arc::new(layout))
    }
}