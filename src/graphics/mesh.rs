use std::sync::Arc;
use std::sync::atomic::{ AtomicU32, Ordering };

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
        return &self.vertex_data;
    }

    pub fn index_data(&self) -> &Vec<u32> {
        return &self.index_data;
    }

    pub fn num_indices(&self) -> u32 {
        self.index_data.len() as u32
    }
}

impl ResourceDescriptor for MeshData {
    type Key = u32;

    fn get_key(&self) -> &Self::Key { &self.id }
}

#[derive(Clone)]
pub struct Mesh {
    pub name: String,
    pub data: Arc<MeshData>,
}

impl Mesh {
    pub fn new(name: &str, data: Arc<MeshData>) -> Self {
        Self {
           name: name.to_string(),
           data
        }
    }
}