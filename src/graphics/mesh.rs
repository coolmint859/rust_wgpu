use std::sync::Arc;
use std::sync::atomic::{ AtomicU32, Ordering };
use wgpu::util::DeviceExt;

use crate::graphics::gpu_resource::ResourceBuilder;

use super::{
    material::Material,
    render_pipeline::RenderPipelineBuilder,
    transform::Transform,
    vertex::Vertex,
};

/// represents a mesh as it lives on the gpu during rendering, most importantly it's buffers
pub struct MeshBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

static DATA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Represents vertex and index data as it lives in cpu memory
#[derive(Debug)]
pub struct MeshData {
    id: u32,
    label: String,
    vertex_data: Vec<Vertex>,
    index_data: Vec<u32>,
}

impl MeshData {
    pub fn new(vertex_data: Vec<Vertex>, index_data: Vec<u32>) -> Self {
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
        let vertex_data = self.vertex_data.to_vec();
        let index_data = self.index_data.to_vec();
        MeshData::new(vertex_data, index_data)
    }
}

impl ResourceBuilder for Arc<MeshData> {
    type Key = u32;
    type Output = MeshBuffer;

    fn get_key(&self) -> u32 {
        self.id
    }

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("Vertex Buffer [{}]", self.label).as_str()),
                contents: bytemuck::cast_slice(self.vertex_data.as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("Index Buffer [{}]", self.label).as_str()),
                contents: bytemuck::cast_slice(self.index_data.as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        println!("Created vertex/index buffer '{}'", self.label);

        Ok(MeshBuffer {
            vertex_buffer,
            index_buffer,
            num_indices: self.index_data.len() as u32,
        })
    }
}

pub struct Mesh<M: Material> {
    pub transform: Transform,
    pub data: Arc<MeshData>,
    pub material: M,
    pub pipeline: RenderPipelineBuilder,
}

impl<M: Material> Mesh<M> {
    pub fn new(
        data: Arc<MeshData>, 
        material: M, 
        pipeline: RenderPipelineBuilder
    ) -> Self {
        Self {
           transform: Transform::default(),
           data: Arc::clone(&data),
           material,
           pipeline,
        }
    }

    /// Create a shallow copy of this mesh (does not duplicate vertex/index or material data)
    pub fn duplicate(&self) -> Mesh<M> {
        let mut mesh_dup = Mesh::new(
            Arc::clone(&self.data),
            self.material.clone(),
            self.pipeline.clone()
        );
        mesh_dup.transform = self.transform.clone();
        
        mesh_dup
    }
}