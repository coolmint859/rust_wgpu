#![allow(dead_code)]
use std::sync::Arc;
use std::sync::atomic::{ AtomicU32, Ordering };

use super::{
    vertex::UV_Vertex,
    handler::ResourceBuilder,
    buffer::BufferBuilder,
};

/// represents a mesh as it lives on the gpu during rendering, most importantly it's buffers
pub struct MeshBuffer {
    pub vertex_buffer: Arc<wgpu::Buffer>,
    pub index_buffer: Arc<wgpu::Buffer>,
    pub num_indices: u32,
}

static DATA_COUNTER: AtomicU32 = AtomicU32::new(0);
static MESH_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Represents vertex and index data as it lives in cpu memory
#[derive(Clone, Debug)]
pub struct MeshData {
    id: u32,
    label: String,
    vertex_data: Vec<UV_Vertex>,
    index_data: Vec<u32>,
}

impl MeshData {
    pub fn new(vertex_data: Vec<UV_Vertex>, index_data: Vec<u32>) -> Self {
        let id = DATA_COUNTER.fetch_add(1, Ordering::SeqCst);

        Self { label: "{id}".to_string(), id, vertex_data,  index_data }
    }

    /// Add a custom label to this Mesh data for GPU profiling.
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Create a deep copy of this MeshData object. This is a blocking call, 
    /// so for large vertex data it is best to wrap this in an async block.
    /// 
    /// Note: The resulting duplicate will not refer to the same buffer data on the gpu.
    /// This is handled automatically by the renderer, but it is worth consideration.
    pub fn duplicate(&self) -> MeshData  {
        MeshData::new(
            self.vertex_data.to_vec(), 
            self.index_data.to_vec()
        )
    }

    pub fn id(&self) -> u32 { self.id.clone() }
}

/// Allows us to treat the mesh data as if it was a regular resource builder
impl ResourceBuilder for Arc<MeshData> {
    type Output = MeshBuffer;
    type Context = wgpu::Device;

    fn build(&self, device: Arc<wgpu::Device>) -> Result<MeshBuffer, String> {
        let vertex_data: Vec<u8> = bytemuck::cast_slice(&self.vertex_data).to_vec();
        let index_data: Vec<u8> = bytemuck::cast_slice(&self.index_data).to_vec();

        let vertex_buffer = BufferBuilder::as_vertex(0)
            .with_label(&format!("vertex_{}", self.id))
            .with_data(vertex_data)
            .build(Arc::clone(&device))?;

        let index_buffer = BufferBuilder::as_index(0)
            .with_label(&format!("index_{}", self.id))
            .with_data(index_data)
            .build(Arc::clone(&device))?;

        println!("[Mesh Data] Created new mesh data with id #{}", self.id);

        Ok(MeshBuffer {
            vertex_buffer,
            index_buffer,
            num_indices: self.index_data.len() as u32
        })
    }
}

pub struct Mesh {
    label: String,
    id: u32,
    data: Arc<MeshData>,
}

impl Mesh {
    pub fn new(label: &str, data: Arc<MeshData>) -> Self {
        let id = MESH_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            label: label.to_string(),
            data: Arc::clone(&data),
        }
    }

    /// Get the unique key of this mesh (label + id + data id).
    pub fn get_key(&self) -> String {
        format!("{}_{}_{}", self.label, self.id, self.data.id)
    }

    /// Get the id of the vertex/index data that this mesh uses
    pub fn get_data_key(&self) -> u32 {
        self.data.id
    }

    /// Create a shallow copy of this mesh (does not duplicate vertex/index data)
    pub fn duplicate(&self) -> Mesh {
        Mesh::new(&self.label.clone(), Arc::clone(&self.data))
    }

    /// Create a shallow copy of this mesh, but with a new label (does not duplicate vertex/index data)
    pub fn duplicate_with_label(&self, label: &str) -> Mesh {
        Mesh::new(label, Arc::clone(&self.data))
    }

    pub fn get_data_builder(&self) -> Arc<MeshData> {
        Arc::clone(&self.data)
    }
}