use std::{collections::HashMap, sync::Arc};
use wgpu::util::DeviceExt;

use crate::graphics::gpu_resource::ResourceBuilder;

/// Holds the bind_group and corresponding buffers
pub struct UniformBuffer {
    pub buffers: HashMap<u32, wgpu::Buffer>,
    pub bind_group: wgpu::BindGroup,
}

/// Represents a single uniform on the gpu
#[derive(Clone, Debug)]
pub struct UniformEntry {
    pub bind_slot: u32,
    pub data: Vec<u8>,
}

/// Represents a group of uniform data to be send to the gpu
#[derive(Clone, Debug)]
pub struct BindGroupBuilder {
    pub key: String,
    pub contents: Vec<UniformEntry>,
    pub bind_layout: Arc<wgpu::BindGroupLayout>,
}

impl BindGroupBuilder {
    /// Takes a POD struct and converts it into a 16-byte aligned byte vector.
    pub fn pad_uniform<T: bytemuck::Pod>(uniform: T) -> Vec<u8> {
        let raw_bytes = bytemuck::bytes_of(&uniform);
        let mut padded_bytes = raw_bytes.to_vec();
        
        // Calculate how many bytes we need to reach the next multiple of 16
        let padding_needed = (16 - (raw_bytes.len() % 16)) % 16;
        
        if padding_needed > 0 {
            padded_bytes.extend(std::iter::repeat(0).take(padding_needed));
        }
        
        padded_bytes
    }
}

impl ResourceBuilder for Arc<BindGroupBuilder> {
    type Key = String;
    type Output = UniformBuffer;

    fn get_key(&self) -> Self::Key {
        self.key.clone()
    }

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        let mut buffers = HashMap::new();
        for entry in &self.contents {
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
                layout: &self.bind_layout,
                entries: &group_entries
            }
        );

        println!("Created uniform buffer & bind group '{}'", self.key);

        Ok(UniformBuffer {
            buffers,
            bind_group
        })
    }
}