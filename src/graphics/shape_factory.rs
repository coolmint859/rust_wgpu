use std::{collections::HashMap, sync::Arc};

use crate::graphics::{geometry::{GeometryBuilder, Vertex}};

/// Creates and stores 2D shapes
pub struct Shape2D {
    shapes: HashMap<String, Arc<GeometryBuilder>>
}

impl Shape2D {
    pub fn new() -> Self {
        Self { shapes: HashMap::new() }
    }

    /// Get or create a square geometry builder
    pub fn square(&mut self) -> Arc<GeometryBuilder> {
        let label = "square".to_string();

        return match self.shapes.get(&label) {
            Some(geometry) => Arc::clone(geometry),
            None => {
                let square = Arc::new(gen_square(&label));
                self.shapes.insert(label.clone(), square.clone());

                Arc::clone(&square)
            }
        }
    }
}

pub fn gen_square(label: &str) -> GeometryBuilder {
    let vertex_data = vec![
        Vertex { position: Some([ 1.0,  1.0, 0.0 ]), uv: Some([1.0, 0.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([-1.0,  1.0, 0.0 ]), uv: Some([0.0, 0.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([-1.0, -1.0, 0.0 ]), uv: Some([0.0, 1.0]), normal: Some([0.0, 0.0, 1.0])},
        Vertex { position: Some([ 1.0, -1.0, 0.0 ]), uv: Some([1.0, 1.0]), normal: Some([0.0, 0.0, 1.0])},
    ];
    let indices =vec![0, 1, 2, 2, 3, 0];

    let builder = GeometryBuilder::new()
        .with_label(label)
        .with_vertices(vertex_data)
        .with_indices(indices);

    builder
}