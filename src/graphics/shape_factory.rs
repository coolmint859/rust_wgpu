use std::{collections::HashMap, sync::Arc};

use glam::{Vec2, Vec3};

use crate::graphics::{data_utils::DirtyVec, geometry::{GeometryData, POSITION_ATTR, UV_ATTR}};

/// Creates and stores 2D shapes
pub struct Shape2D {
    shapes: HashMap<String, Arc<GeometryData>>
}

impl Shape2D {
    pub fn new() -> Self {
        Self { shapes: HashMap::new() }
    }

    /// Get or create a square geometry builder
    pub fn square(&mut self) -> Arc<GeometryData> {
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

pub fn gen_square(label: &str) -> GeometryData {
    let positions = DirtyVec::from_vec(vec![
        Vec3::new( 1.0,  1.0, 0.0 ),
        Vec3::new(-1.0,  1.0, 0.0 ),
        Vec3::new(-1.0, -1.0, 0.0 ),
        Vec3::new( 1.0, -1.0, 0.0 ),
    ]);

    let uvs = DirtyVec::from_vec( vec![
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
    ]);

    let indices =vec![0, 1, 2, 2, 3, 0];

    GeometryData::new(positions.inner.len())
        .with_label(label)
        .with_attr(POSITION_ATTR, positions)
        .with_attr(UV_ATTR, uvs)
        .with_indices(indices)
}