#![allow(dead_code)]
use std::sync::Arc;
use crate::graphics::handler::ResourceBuilder;

#[derive(Clone, Debug)]
pub enum BufferType {
    Uniform,
    Storage,
    Vertex,
    Index,
}

pub struct BufferContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>
}

/// Constructs a buffer on the gpu with the provided data, if any
#[derive(Clone, Debug)]
pub struct BufferBuilder {
    label: String,
    usage: BufferType,
    size: usize,
    data: Vec<u8>,
}

impl BufferBuilder {
    pub fn new(usage: BufferType) -> Self {
        Self {
            label: "buffer-builder".to_string(),
            data: Vec::new(),
            size: 0,
            usage
        }
    }

    /// Create a buffer builder that produces a uniform buffer
    pub fn as_uniform() -> Self {
        BufferBuilder::new(BufferType::Uniform)
    }

    /// Create a buffer builder that produces a storage buffer
    pub fn as_storage() -> Self {
        BufferBuilder::new(BufferType::Storage)
    }

    /// Create a buffer builder that produces a vertex buffer
    pub fn as_vertex() -> Self {
        BufferBuilder::new(BufferType::Vertex)
    }

    /// Create a buffer builder that produces an index buffer
    pub fn as_index() -> Self {
        BufferBuilder::new(BufferType::Index,)
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add a maximum capacity for the buffer.
    /// 
    /// If data is added prior to a call to build() that is larger than the capacity, 
    /// the capacity is ignored and the builder will allocate enough space for the data.
    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.size = capacity;
        self
    }

    /// Add data to the buffer from a byte vector
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Add data to the buffer in the form of a POD struct
    pub fn with_data_from_struct<T: bytemuck::Pod>(mut self, data_struct: T) -> Self {
        self.data = BufferBuilder::to_padded_vec(data_struct);
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
    type Context = BufferContext;

    fn build(&self, context: Arc<BufferContext>) -> Result<Arc<wgpu::Buffer>, String> {
        let buffer_size = self.size.max(self.data.len());

        let buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&self.label),
            size: buffer_size as u64,
            usage: BufferBuilder::buffer_usage(&self.usage),
            mapped_at_creation: false,
        });

        println!("[Buffer] Created new buffer of type {:?} with label '{}'", self.usage, self.label);
        
        if self.data.len() > 0 {
            context.queue.write_buffer(&buffer, 0, &self.data);
        }

        Ok(Arc::new(buffer))
    }
}