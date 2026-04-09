#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{
    bind_group::{BindGroupLayoutBuilder, LayoutVisibility}, 
    mesh::MeshData, 
    registry::ResourceRegistry, 
    render_pipeline::RenderPipelineBuilder, 
    vertex::PositionVertex,
};

/// Preset rendering pipelines
pub enum Pipeline {
    /// Simple 2D colored sprite rendering pipeline
    ColoredSprite,
}

impl Pipeline {
    /// Returns a RenderPipelineBuilder corresponding to the pipeline preset variant
    pub fn get(self) -> RenderPipelineBuilder {
        return match self {
            Pipeline::ColoredSprite => {
                RenderPipelineBuilder::new()
                    .with_label("colored-sprite")
                    .with_shader("src/graphics/shaders/shader.wgsl")
                    .with_vertex_layout::<PositionVertex>()
                    .with_bg_layout("camera-2d")
                    .with_bg_layout("colored-sprite")
                    .with_label("colored-sprite")
            }
        }
    }
}

pub enum BindingLayout {
    ColoredSprite,
    TexturedSprite,
    Camera2D,
}

impl BindingLayout {
    pub fn get(self) -> BindGroupLayoutBuilder {
        return match self {
            BindingLayout::ColoredSprite => {
                BindGroupLayoutBuilder::new()
                    .with_label("colored-sprite")
                    .with_uniform_entry(LayoutVisibility::VertexFragment)
            },
            BindingLayout::TexturedSprite => {
                BindGroupLayoutBuilder::new()
                    .with_label("textured-sprite")
                    .with_uniform_entry(LayoutVisibility::Fragment)
            },
            BindingLayout::Camera2D => {
                BindGroupLayoutBuilder::new()
                    .with_label("camera-2d")
                    .with_uniform_entry(LayoutVisibility::VertexFragment)
            }
        }
    }
}

pub struct Shape2D {
    shape_data: ResourceRegistry<String, Arc<MeshData>>,
}

impl Shape2D {
    pub fn new() -> Self {
        Self { shape_data: ResourceRegistry::new() }
    }

    /// Generate mesh data for a triangle
    pub fn triangle(&mut self) -> Arc<MeshData> {
        let key = "triangle".to_string();

        return match self.shape_data.get(&key) {
            Some(data) => Arc::clone(data),
            None => {
                self.shape_data.store(
                    &key, 
                    Arc::new(gen_triangle())
                );
                Arc::clone(self.shape_data.get(&key).unwrap())
            }
        }
    }

    /// Generate mesh data for a square.
    pub fn square(&mut self) -> Arc<MeshData> {
        let key = "square".to_string();
        
        return match self.shape_data.get(&key) {
            Some(data) => Arc::clone(data),
            None => {
                self.shape_data.store(
                    &key, 
                    Arc::new(gen_square())
                );
                Arc::clone(self.shape_data.get(&key).unwrap())
            }
        }
    }

    // /// Generate mesh data for a polygon
    // pub fn polygon(&mut self, num_sides: u32)  -> Arc<MeshData> {
    //     let key = format!("polygon{}", num_sides);
        
    //     return match self.shape_data.get(&key) {
    //         Some(data) => Arc::clone(data),
    //         None => {
    //             self.shape_data.store(
    //                 &key, 
    //                 Arc::new(gen_square())
    //             );
    //             Arc::clone(self.shape_data.get(&key).unwrap())
    //         }
    //     }
    // }
}

/// Get raw triangle data
pub fn gen_triangle() -> MeshData {
    MeshData::new(
        vec![
            PositionVertex { position: [0.0, 0.5, 0.0] },
            PositionVertex { position: [-0.5, -0.5, 0.0] },
            PositionVertex { position: [0.5, -0.5, 0.0] },
        ],
        vec![0, 1, 2]
    )
    .with_label("triangle")
}

/// Get raw square data
pub fn gen_square() -> MeshData  {
    MeshData::new(
        vec![
            PositionVertex { position: [ 0.5,  0.5, 0.0] },
            PositionVertex { position: [-0.5,  0.5, 0.0] },
            PositionVertex { position: [-0.5, -0.5, 0.0] },
            PositionVertex { position: [ 0.5, -0.5, 0.0] },
        ],
        vec![
            0, 1, 2,
            2, 3, 0
        ]
    )
    .with_label("square")
}

// /// Get raw polygon data
// pub fn gen_polygon() -> MeshData {

// }