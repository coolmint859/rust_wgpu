use std::{collections::HashMap, sync::Arc};
use wgpu::util::DeviceExt;

use crate::graphics::{
    material::UniformGroup, registry::{ResourceRegistry, ResourceStatus}, traits::{Handler, ResourceDescriptor}
};

pub struct UniformBuffer {
    pub buffers: HashMap<u32, wgpu::Buffer>,
    pub bind_group: wgpu::BindGroup,
}

impl UniformBuffer {
    pub async fn new(
        device: Arc<wgpu::Device>, 
        uniform: Arc<UniformGroup>,
    ) -> Result<UniformBuffer, String> {
        let mut buffers = HashMap::new();
        for entry in &uniform.contents {
            let buffer = device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Uniform Buffer; slot #{}", entry.bind_slot)),
                    contents: &entry.data,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }
            );
            buffers.insert(entry.bind_slot, buffer);
        }

        let mut group_entries: Vec<wgpu::BindGroupEntry> = Vec::new();
        for (bind_slot, buffer) in buffers.iter() {
            group_entries.push(wgpu::BindGroupEntry {
                binding: *bind_slot,
                resource: buffer.as_entire_binding()
            });
        }

        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("Model Bind Group"),
                layout: &uniform.bind_layout,
                entries: &group_entries
            }
        );

        println!("Created uniform buffer & bind group '{}'", uniform.key);

        Ok(Self {
            buffers,
            bind_group
        })
    } 
}

pub struct UniformBufferHandler {
    device: Arc<wgpu::Device>,
    buffers: ResourceRegistry<String, UniformBuffer>,
}

impl UniformBufferHandler {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            device,
            buffers: ResourceRegistry::new(),
        }
    }
}

impl Handler<UniformGroup, UniformBuffer> for UniformBufferHandler {
    fn request_new(&mut self, uniform: Arc<UniformGroup>) {
        let uniform_cpy = Arc::clone(&uniform);
        let device_cpy = Arc::clone(&self.device);
        self.buffers.request_new(uniform.get_key(), UniformBuffer::new(device_cpy, uniform_cpy));
    }
    
    fn request_wait(&mut self, uniform: Arc<UniformGroup>) {
        let device_cpy = Arc::clone(&self.device);
        let uniform_cpy = Arc::clone(&uniform);

        let result = self.buffers.request_wait(
            uniform.get_key(),
            UniformBuffer::new(device_cpy, uniform_cpy),
        );

        match result {
            Err(e) => eprintln!("Error creating mesh buffers: {e}"),
            _ => {}
        }
    }

    fn sync(&mut self) {
        self.buffers.sync();
    }

    fn get(&self, buffer_key: &String) -> Option<&UniformBuffer> {
        self.buffers.get(buffer_key)
    }

    fn remove(&mut self, buffer_key: &String) {
        self.buffers.remove(buffer_key);
    }

    fn contains(&self, buffer_key: &String) -> bool {
        self.buffers.contains(buffer_key)
    }

    fn status_of(&self, buffer_key: &String) -> Option<&ResourceStatus<UniformBuffer>> {
        return self.buffers.status_of(buffer_key);
    }
}