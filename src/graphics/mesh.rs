use std::sync::Arc;
use std::sync::atomic::{ AtomicU32, Ordering };

use crate::graphics::material::Material;
use crate::graphics::presets::RenderPipelineConfig;

use super::transform::Transform;

use super::vertex::Vertex;
use super::traits::ResourceDescriptor;

static DATA_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug)]
pub struct MeshData {
    id: u32,
    vertex_data: Vec<Vertex>,
    index_data: Vec<u32>,
}

impl MeshData {
    pub fn new(vertex_data: Vec<Vertex>, index_data: Vec<u32>) -> Self {
        let id = DATA_COUNTER.fetch_add(1, Ordering::SeqCst);

        Self {
            id, 
            vertex_data, 
            index_data,
        }
    }

    pub fn vertex_data(&self) -> &Vec<Vertex> {
        &self.vertex_data
    }

    pub fn index_data(&self) -> &Vec<u32> {
        &self.index_data
    }

    pub fn num_indices(&self) -> u32 {
        self.index_data.len() as u32
    }
}

impl ResourceDescriptor for MeshData {
    type Key = u32;

    fn get_key(&self) -> &Self::Key { &self.id }
}

pub struct Mesh<M: Material + Clone> {
    pub transform: Transform,
    pub data: Arc<MeshData>,
    pub material: M,
    pub pipeline: RenderPipelineConfig,
}

impl<M: Material + Clone> Mesh<M> {
    pub fn new(
        data: Arc<MeshData>, 
        material: M, 
        pipeline: RenderPipelineConfig
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