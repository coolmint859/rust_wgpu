use std::sync::Arc;
use std::sync::atomic::{ AtomicU32, Ordering };

use super::{
    material::Material,
    transform::Transform,
    vertex::PositionVertex,
    render_pipeline::RenderPipelineTemplate,
    gpu_resource::ResourceBuilder,
    buffer::BufferBuilder,
    bind_group::ResourceID
};

/// represents a mesh as it lives on the gpu during rendering, most importantly it's buffers
pub struct MeshBuffer {
    pub vertex_buffer: Arc<wgpu::Buffer>,
    pub index_buffer: Arc<wgpu::Buffer>,
    pub num_indices: u32,
}

static DATA_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Represents vertex and index data as it lives in cpu memory
#[derive(Clone, Debug)]
pub struct MeshData {
    id: u32,
    label: String,
    vertex_data: Vec<PositionVertex>,
    index_data: Vec<u32>,
}

impl MeshData {
    pub fn new(vertex_data: Vec<PositionVertex>, index_data: Vec<u32>) -> Self {
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
            .with_label(&format!("mesh #{}: vertex", self.id))
            .with_data(vertex_data)
            .build(Arc::clone(&device))?;

        let index_buffer = BufferBuilder::as_index(0)
            .with_label(&format!("mesh #{}: index", self.id))
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

pub struct Mesh<M: Material> {
    pub transform: Transform,
    pub data: Arc<MeshData>,
    pub material: M,
    pub pipeline: RenderPipelineTemplate,
}

impl<M: Material> Mesh<M> {
    pub fn new(
        data: Arc<MeshData>, 
        material: M, 
        pipeline: RenderPipelineTemplate
    ) -> Self {
        Self {
           transform: Transform::default(),
           data: Arc::clone(&data),
           material,
           pipeline,
        }
    }

    /// get the required buffers and their keys needed to render the mesh.
    pub fn get_requirements(&self) -> Vec<(ResourceID, BufferBuilder)> {
        let material_data = self.material.get_data(self.transform.world_matrix());

        let mat_key= self.material.get_key();

        let key = ResourceID { group_id: mat_key.clone(), binding: 0 };
        let builder = BufferBuilder::as_uniform( 0)
            .with_label(&format!("mesh #{}:{}", self.data.id, mat_key))
            .with_data(material_data);

        return vec![(key, builder)]
    }

    /// Check for internal updates and return key-value pairs of updated resources.
    /// 
    /// This is a destructive read, so subsequent calls in the same frame will yeild an empty vector
    pub fn get_updated(&mut self) -> Vec<(ResourceID, Vec<u8>)> {
        if self.material.is_dirty() {
            self.material.update_state();
        }
        if self.transform.is_dirty() {
            self.transform.update_world_mat();
        }

        let key = ResourceID { group_id: self.material.get_key(), binding: 0 };
        let data = self.material.get_data(self.transform.world_matrix());
        vec![(key, data)]
    }

    /// Retrieve this mesh's resource keys as vector of ResourceIDs
    pub fn get_resource_keys(&self) -> Vec<ResourceID> {
        vec![ResourceID { group_id: self.material.get_key(), binding: 0 }]
    }

    /// Create a shallow copy of this mesh (does not duplicate vertex/index data)
    pub fn duplicate(&self) -> Mesh<M> {
        let mut mesh_dup = Mesh::new(
            Arc::clone(&self.data),
            self.material.clone(),
            self.pipeline.clone()
        );
        mesh_dup.transform = self.transform.clone();
        
        mesh_dup
    }

    pub fn get_data_builder(&self) -> Arc<MeshData> {
        Arc::clone(&self.data)
    }

    pub fn get_pipeline(&self) -> RenderPipelineTemplate {
        self.pipeline.clone()
    }
}