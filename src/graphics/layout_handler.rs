use std::sync::Arc;

use super::{
    registry::{ ResourceStatus, ResourceRegistry }, 
    traits::{ ResourceDescriptor, Handler }
};


#[derive(Clone, Debug)]
/// Represents a Bind Group Layout for wgpu
pub struct LayoutConfig {
    pub key: String,
    pub bind_group: u32,
    pub entries: Vec<LayoutEntry>
}

#[derive(Clone, Debug)]
/// Represents a single bind group layout entry
pub struct LayoutEntry {
    pub binding: u32,
    pub visibility: wgpu::ShaderStages,
    pub ty: BindingType,
}

#[derive(Clone, Debug)]
/// Bind Group Entry layout types
pub enum BindingType {
    Uniform,
    Storage,
    Texture,
    Sampler,
}

impl ResourceDescriptor for LayoutConfig {
    type Key = String;

    fn get_key(&self) -> &Self::Key { &self.key }
}

/// Handles bind group layouts used for creating wgpu pipelines and buffers
pub struct LayoutHandler {
    device: Arc<wgpu::Device>,
    layouts: ResourceRegistry<String, Arc<wgpu::BindGroupLayout>>
}

impl LayoutHandler {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            device,
            layouts: ResourceRegistry::new(),
        }
    }

    /// Create a new bind group layout based on a layout config
    async fn create_binding_layout(
        device: Arc<wgpu::Device>, 
        desc: Arc<LayoutConfig>
    ) -> Result<Arc<wgpu::BindGroupLayout>, String> {
        let mut group_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();

        for entry in &desc.entries {
            group_entries.push(wgpu::BindGroupLayoutEntry {
                binding: entry.binding,
                visibility: entry.visibility,
                ty: match entry.ty {
                    BindingType::Uniform => wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Uniform, 
                        has_dynamic_offset: false, 
                        min_binding_size: None
                    },
                    BindingType::Storage => wgpu::BindingType::Buffer { 
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
            label: Some(format!("[Bind Group Layout] @{}", &desc.bind_group).as_str()),
            entries: &group_entries,
        });

        println!("Created bind group layout '{}'", desc.key);

        Ok(Arc::new(layout))
    }
}

impl Handler<LayoutConfig, Arc<wgpu::BindGroupLayout>> for LayoutHandler {
    fn request_new(&mut self, desc: Arc<LayoutConfig>) {
        let desc_cpy = Arc::clone(&desc);
        let device_cpy = Arc::clone(&self.device);

        self.layouts.request_new(
            desc.get_key(), 
            LayoutHandler::create_binding_layout(device_cpy, desc_cpy)
        );
    }

    fn request_wait(&mut self, desc: Arc<LayoutConfig>) {
        let desc_cpy = Arc::clone(&desc);
        let device_cpy = Arc::clone(&self.device);

        let result = self.layouts.request_wait(
            desc.get_key(), 
            LayoutHandler::create_binding_layout(device_cpy, desc_cpy)
        );

        match result {
            Err(e) => eprintln!("Error creating bind group layout: {e}"),
            _ => {}
        }
    }

    fn contains(&self, key: &String) -> bool {
        self.layouts.contains(key)
    }

    fn get(&self, key: &String) -> Option<&Arc<wgpu::BindGroupLayout>> {
        self.layouts.get(key)
    }

    fn remove(&mut self, key: &String) {
        self.layouts.remove(key);
    }

    fn sync(&mut self) {
        self.layouts.sync();
    }

    fn status_of(&self, key: &String) -> Option<&ResourceStatus<Arc<wgpu::BindGroupLayout>>> {
        self.layouts.status_of(key)
    }
}