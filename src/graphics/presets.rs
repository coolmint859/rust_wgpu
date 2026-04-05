#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{
    bind_group_layout::{BindingType, LayoutConfig, LayoutEntry}, 
    mesh::MeshData, 
    registry::ResourceRegistry, 
    render_pipeline::RenderPipelineBuilder, 
    vertex::Vertex,
    wpgu_context::{GLOBAL_UNIFORMS, MATERIAL_UNIFORMS}
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
                let builder = RenderPipelineBuilder::new("colored-sprite")
                    .with_shader("src/graphics/shaders/shader.wgsl")
                    .with_vertex_layout::<Vertex>()
                    .with_layout("camera-2d")
                    .with_layout("colored-sprite");

                builder
            }
        }
    }
}

pub enum BindingLayout {
    ColoredSprite,
    Camera2D,
}

impl BindingLayout {
    pub fn get(self) -> Arc<LayoutConfig> {
        return match self {
            BindingLayout::ColoredSprite => {
                let material = LayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: BindingType::Uniform
                };

                Arc::new(LayoutConfig { 
                    key: "colored-sprite".to_string(),
                    bind_group: MATERIAL_UNIFORMS, 
                    entries: vec![material]
                })
            },
            BindingLayout::Camera2D => {
                let camera = LayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: BindingType::Uniform
                };

                Arc::new(LayoutConfig { 
                    key: "camera-2d".to_string(),
                    bind_group: GLOBAL_UNIFORMS, 
                    entries: vec![camera]
                })
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
            Vertex { position: [0.0, 0.5, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.0] },
            Vertex { position: [0.5, -0.5, 0.0] },
        ],
        vec![0, 1, 2]
    )
}

/// Get raw square data
pub fn gen_square() -> MeshData  {
    MeshData::new(
        vec![
            Vertex { position: [ 0.5,  0.5, 0.0] },
            Vertex { position: [-0.5,  0.5, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.0] },
            Vertex { position: [ 0.5, -0.5, 0.0] },
        ],
        vec![
            0, 1, 2,
            2, 3, 0
        ]
    )
}

// /// Get raw polygon data
// pub fn gen_polygon() -> MeshData {

// }