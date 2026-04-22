use std::sync::Arc;

use crate::graphics::{handler::{ResourceBuilder, ResourceHandler}, mesh::MeshData, vertex::{VertexAttribute, VertexLayoutBuilder}};

/// Represents a single vertex in a mesh
pub struct Vertex {
    pub position: Option<[f32; 3]>,
    pub uv: Option<[f32; 2]>,
    pub normal: Option<[f32; 3]>
}

/// Constructs a byte vector representing vertex data
#[derive(Clone, Debug)]
pub struct GeometryBuilder {
    vertex_data: Arc<Vec<u8>>,
    vertex_layout: VertexLayoutBuilder,
}

impl GeometryBuilder {
    pub fn new(vertex_layout: VertexLayoutBuilder) -> Self {
        Self {
            vertex_data: Arc::new(Vec::new()),
            vertex_layout,
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) {
        if let Some(vertex_data) = Arc::get_mut(&mut self.vertex_data) {
            for attr in self.vertex_layout.get_attributes() {
                match attr {
                    VertexAttribute::Position => {
                        if let Some(position) = vertex.position {
                            vertex_data.extend_from_slice(bytemuck::cast_slice(&position));
                        }
                    },
                    VertexAttribute::Normal => {
                        if let Some(normal) = vertex.normal {
                            vertex_data.extend_from_slice(bytemuck::cast_slice(&normal));
                        }
                    },
                    VertexAttribute::UV => {
                        if let Some(uv) = vertex.uv {
                            vertex_data.extend_from_slice(bytemuck::cast_slice(&uv));
                        }
                    }
                }
            }
        } else {
            panic!("Cannot add vertex data as the internal vector is being referenced elsewhere!");
        }
    }
}

impl ResourceBuilder for GeometryBuilder {
    type Context = ();
    type Output = Arc<Vec<u8>>;

    fn build(&self, _context: Arc<Self::Context>) -> Result<Self::Output, String> {
        Ok(Arc::clone(&self.vertex_data))
    }
}

/// key for mesh data with certain attributes
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct VertexID {
    key: String,
    vertex_layout: VertexLayoutBuilder
}

/// Creates and stores 2D shapes
pub struct Shape2D {
    shapes: ResourceHandler<VertexID, Arc<MeshData>>
}

impl Shape2D {
    pub fn new() -> Self {
        Self { shapes: ResourceHandler::new() }
    }

    /// Get or create a square mesh data using the provided layout builder
    pub fn square(&mut self, vertex_layout: VertexLayoutBuilder) -> Arc<MeshData> {
        let id = VertexID {
            key: "square".to_string(),
            vertex_layout: vertex_layout.clone()
        };

        return match self.shapes.get(&id) {
            Some(data) => Arc::clone(data),
            None => {
                let square = Arc::new(gen_square(vertex_layout));
                self.shapes.store(&id, Arc::clone(&square));
                Arc::clone(&square)
            }
        }
    }
}

pub fn gen_square(vertex_layout: VertexLayoutBuilder) -> MeshData {
    let vertex_data = [
        Vertex { position: Some([ 1.0,  1.0, 0.0 ]), uv: Some([1.0, 0.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([-1.0,  1.0, 0.0 ]), uv: Some([0.0, 0.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([-1.0, -1.0, 0.0 ]), uv: Some([0.0, 1.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([ 1.0, -1.0, 0.0 ]), uv: Some([1.0, 1.0]), normal: Some([0.0, 0.0, 1.0])},
    ];

    let mut builder = GeometryBuilder::new(vertex_layout);
    for vertex in vertex_data {
        builder.add_vertex(vertex);
    }

    MeshData::new(
        builder.build(().into()).unwrap(), 
        Arc::new(vec![
            0, 1, 2,
            2, 3, 0
        ])
    )
}