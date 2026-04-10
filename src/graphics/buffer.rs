#![allow(dead_code)]
use std::sync::Arc;
use crate::graphics::gpu_resource::ResourceBuilder;
use wgpu::util::DeviceExt;

#[derive(Clone, Debug)]
pub enum BufferType {
    Uniform,
    Storage,
    Vertex,
    Index,
}

/// Constructs a buffer on the gpu with the provided data, if any
#[derive(Clone, Debug)]
pub struct BufferBuilder {
    label: String,
    usage: BufferType,
    size: u64,
    data: Option<Vec<u8>>,
}

impl BufferBuilder {
    pub fn new(usage: BufferType, size: u64) -> Self {
        Self {
            label: "buffer-builder".to_string(),
            data: None,
            size,
            usage
        }
    }

    /// Create a buffer builder that produces a uniform buffer
    pub fn as_uniform(size: u64) -> Self {
        BufferBuilder::new(BufferType::Uniform, size)
    }

    /// Create a buffer builder that produces a storage buffer
    pub fn as_storage(size: u64) -> Self {
        BufferBuilder::new(BufferType::Storage, size)
    }

    /// Create a buffer builder that produces a vertex buffer
    pub fn as_vertex(size: u64) -> Self {
        BufferBuilder::new(BufferType::Vertex, size)
    }

    /// Create a buffer builder that produces an index buffer
    pub fn as_index(size: u64) -> Self {
        BufferBuilder::new(BufferType::Index, size)
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add data to the buffer from a byte vector
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = Some(data);
        self
    }

    /// Add data to the buffer in the form of a POD struct
    pub fn with_data_from_struct<T: bytemuck::Pod>(mut self, data_struct: T) -> Self {
        self.data = Some(BufferBuilder::to_padded_vec(data_struct));
        self
    }

    /// Takes a POD struct and converts it into a 16-byte aligned byte vector.
    pub fn to_padded_vec<T: bytemuck::Pod>(data_struct: T) -> Vec<u8> {
        let raw_bytes = bytemuck::bytes_of(&data_struct);
        let mut padded_bytes = raw_bytes.to_vec();
        
        // Calculate how many bytes we need to reach the next multiple of 16
        let padding_needed = (16 - (raw_bytes.len() % 16)) % 16;
        
        if padding_needed > 0 {
            padded_bytes.extend(std::iter::repeat(0).take(padding_needed));
        }
        
        padded_bytes
    }

    fn buffer_usage(ty: &BufferType) -> wgpu::BufferUsages {
        match ty {
            BufferType::Uniform => wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            BufferType::Storage => wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            BufferType::Vertex => wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            BufferType::Index => wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        }
    }
}

impl ResourceBuilder for BufferBuilder {
    type Output = Arc<wgpu::Buffer>;

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        println!("[Buffer] Created new buffer of type {:?} with label '{}'", self.usage, self.label);
        if let Some(data) = &self.data {
            Ok(Arc::new(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&self.label),
                contents: data,
                usage: BufferBuilder::buffer_usage(&self.usage),
            })))
        } else {
            // Otherwise, allocate empty space
            Ok(Arc::new(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&self.label),
                size: self.size,
                usage: BufferBuilder::buffer_usage(&self.usage),
                mapped_at_creation: false,
            })))
        }
    }
}