#![allow(dead_code)]
use std::sync::Arc;
use std::sync::atomic::{ AtomicU32, Ordering };

use crate::graphics::material::UniformBuilder;

use super::{
    material::Material,
    transform::Transform,
    vertex::PositionVertex,
    render_pipeline::RenderPipelineBuilder,
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
            .with_label(&format!("mesh-data#{}-vertex", self.id))
            .with_data(vertex_data)
            .build(Arc::clone(&device))?;

        let index_buffer = BufferBuilder::as_index(0)
            .with_label(&format!("mesh-data#{}-index", self.id))
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

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelMatrixUniform {
    model_matrix: [f32; 16]
}

pub struct Mesh<M: Material> {
    pub id: u32,
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
        let id = MESH_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            transform: Transform::default(),
            data: Arc::clone(&data),
            material,
            pipeline,
        }
    }

    /// Retrieve the uniform keys for this mesh and it's material.
    pub fn get_uniform_keys(&self) -> (String, String) {
        (
        self.get_key(),
        self.material.get_group_key(self.id)
        )
    }

    /// Retrieve the uniform key specific for this mesh.
    pub fn get_key(&self) -> String {
        format!("mesh#{}-instance-uniforms", self.id)
    }

    /// get the mesh's uniform buffer keys and their bind group bindings. (Used to make a bind group)
    pub fn get_material_requirements(&self) -> Vec<(String, u32)> {
        vec![(self.material.get_group_key(self.id), 0)]
    }

    /// get the uniform builders and their keys (used to make uniform buffers)
    pub fn get_uniforms(&mut self) -> Vec<(String, UniformBuilder)> {
        let mut uniforms = self.material.get_uniform_builders(self.id);
        
        // if self.transform.is_dirty() {
            self.transform.update();

            // println!("creating instance uniforms for mesh #{}", self.id);

            let model_mat = ModelMatrixUniform {
                model_matrix: self.transform.world_matrix().to_cols_array()
            };

            let builder: UniformBuilder = UniformBuilder::Buffer(
                BufferBuilder::as_uniform(0)
                    .with_label(&self.get_key())
                    .with_data_from_struct(model_mat)
            );
            uniforms.push((self.get_key(), builder));
        // }
        uniforms
    }

    /// Check for internal updates and return key-value pairs of updated resources.
    /// 
    /// This is a destructive read, so subsequent calls in the same frame will yeild an empty vector
    pub fn get_updated(&mut self) -> Vec<(String, Vec<u8>)> {
        if self.transform.is_dirty() {
            self.transform.update();
        }
        let mut updated = self.material.get_buffers_updated(self.id);
        
        // if self.transform.is_dirty() {
            self.transform.update();

            let model_mat = ModelMatrixUniform {
                model_matrix: self.transform.world_matrix().to_cols_array()
            };

            let model_mat_data = BufferBuilder::to_padded_vec(model_mat);
            updated.push((self.get_key(), model_mat_data));
        // }

        updated
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

    pub fn get_rpip_builder(&self) -> RenderPipelineBuilder {
        self.pipeline.clone()
    }
}