use std::{collections::HashMap, sync::Arc};

use crate::graphics::{geometry::{Geometry, GeometryBuilder, GeometryID, Vertex}, vertex::VertexLayoutBuilder};

/// Creates and stores 2D shapes
pub struct Shape2D {
    shapes: HashMap<GeometryID, Arc<GeometryBuilder>>
}

impl Shape2D {
    pub fn new() -> Self {
        Self { shapes: HashMap::new() }
    }

    /// Get or create a square mesh data using the provided layout builder
    pub fn square(&mut self, vertex_layout: VertexLayoutBuilder) -> Geometry {
        let id = GeometryID {
            label: "square".to_string(),
            vertex_layout: vertex_layout.clone()
        };

        return match self.shapes.get(&id) {
            Some(geometry) => {
                Geometry { id, builder: Arc::clone(geometry) }
            }
            None => {
                let square = Arc::new(gen_square(&id.label, vertex_layout));
                self.shapes.insert(id.clone(), square.clone());

                Geometry { id, builder: Arc::clone(&square) }
            }
        }
    }
}

pub fn gen_square(label: &str, vertex_layout: VertexLayoutBuilder) -> GeometryBuilder {
    let vertex_data = vec![
        Vertex { position: Some([ 1.0,  1.0, 0.0 ]), uv: Some([1.0, 0.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([-1.0,  1.0, 0.0 ]), uv: Some([0.0, 0.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([-1.0, -1.0, 0.0 ]), uv: Some([0.0, 1.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([ 1.0, -1.0, 0.0 ]), uv: Some([1.0, 1.0]), normal: Some([0.0, 0.0, 1.0])},
    ];
    let indices =vec![0, 1, 2, 2, 3, 0];

    let builder = GeometryBuilder::new(vertex_layout)
        .with_label(label)
        .with_vertices(vertex_data)
        .with_indices(indices);

    builder
}