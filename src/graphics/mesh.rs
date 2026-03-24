use std::sync::atomic::{ AtomicU32, Ordering };

use super::vertex::Vertex;
use super::traits::ResourceDescriptor;

static MESH_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
pub struct Mesh {
    id: u32,
    vertex_data: Vec<Vertex>,
    index_data: Vec<u32>,
}

impl ResourceDescriptor for Mesh {
    type Key = u32;

    fn get_key(&self) -> &Self::Key { &self.id }
}

impl Mesh {
    pub fn new(vertex_data: Vec<Vertex>, index_data: Vec<u32>) -> Self {
        let id = MESH_COUNTER.fetch_add(1, Ordering::SeqCst);

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