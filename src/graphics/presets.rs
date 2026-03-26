#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{mesh::MeshData, registry::ResourceRegistry, vertex::Vertex};

/// lightwight configuration struct for WGPU rendering pipelines
#[derive(Clone)]
pub struct RenderPipelineConfig {
    /// unique identifier for the pipeline
    pub name: String,
    /// the path to the pipeline's shader source file
    pub shader_path: String,
    /// the name of the entry function into the vertex stage 
    pub vert_main: String,
    /// the name of the entry function into the fragment stage
    pub frag_main: String,
}

/// Preset rendering pipelines
pub enum Pipeline {
    /// Simple 2D colored sprite rendering pipeline
    ColoredSprite,
}

impl Pipeline {
    /// Returns a RenderPipelineConfig corresponding to the pipeline preset variant
    pub fn get(&self) -> Arc<RenderPipelineConfig> {
        return match *self {
            Pipeline::ColoredSprite => Arc::new(RenderPipelineConfig {
                name: "shader".into(),
                shader_path: "src/graphics/shaders/shader.wgsl".into(),
                vert_main: "vs_main".into(),
                frag_main: "fs_main".into(),
            }),
        }
    }
}

pub struct Shape2D {
    shape_data: ResourceRegistry<String, Arc<MeshData>>,
}

impl Shape2D {
    pub fn new() -> Self {
        Self {
            shape_data: ResourceRegistry::new()
        }
    }

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

pub fn gen_triangle() -> MeshData {
    MeshData::new(
        vec![
            Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
        ],
        vec![0, 1, 2]
    )
}

pub fn gen_square() -> MeshData  {
    MeshData::new(
        vec![
            Vertex { position: [ 0.5,  0.5, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [-0.5,  0.5, 0.0], color: [0.0, 1.0, 0.5] },
            Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
            Vertex { position: [ 0.5, -0.5, 0.0], color: [0.5, 0.0, 0.5] },
        ],
        vec![
            0, 1, 2,
            2, 3, 0
        ]
    )
}

// pub fn gen_polygon() -> MeshData {

// }