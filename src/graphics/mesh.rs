use std::sync::atomic::{ AtomicU32, Ordering };

use crate::graphics::vertex::Vertex;

static MESH_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Clone)]
pub struct Mesh {
    id: u32,
    vertex_data: Vec<Vertex>,
    index_data: Vec<u32>,
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

    pub fn id(&self) -> u32 {
        self.id
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