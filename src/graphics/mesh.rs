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

pub struct Mesh<M: Material> {
    pub name: String,
    pub transform: Transform,
    pub data: Arc<MeshData>,
    pub material: Arc<M>,
    pub pipeline: RenderPipelineConfig,
}

impl<M: Material> Mesh<M> {
    pub fn new(
        name: &str, 
        data: Arc<MeshData>, 
        material: Arc<M>, 
        pipeline: RenderPipelineConfig
    ) -> Self {
        Self {
           name: name.to_string(),
           transform: Transform::default(),
           data: Arc::clone(&data),
           material: Arc::clone(&material),
           pipeline,
        }
    }

    /// Create a shallow copy of this mesh (does not duplicate vertex/index data)
    pub fn duplicate(&self) -> Mesh<M> {
        Mesh {
            name: self.name.clone(),
            data: Arc::clone(&self.data), // reference to internal data
            material: Arc::clone(&self.material),
            transform: self.transform.clone(),
            pipeline: self.pipeline.clone()
        }
    }

    /// Get the key to this mesh
    pub fn get_key(&self) -> String {
        self.name.clone()
    }
}