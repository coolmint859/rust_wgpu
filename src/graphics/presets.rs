#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{
    layout_handler::{BindingType, LayoutConfig, LayoutEntry}, 
    mesh::MeshData, 
    registry::ResourceRegistry,
    vertex::Vertex, wpgu_context::MATERIAL_GROUP
};

/// lightwight configuration struct for WGPU rendering pipelines
#[derive(Clone, Debug)]
pub struct RenderPipelineConfig {
    /// A unique identifier for the pipeline
    pub name: String,
    /// The path to the pipeline's shader source file
    pub shader_path: String,
    /// The name of the entry function into the vertex stage 
    pub vert_main: String,
    /// The name of the entry function into the fragment stage
    pub frag_main: String,
    /// The layout bindings the pipeline should expect (see wgpu_context)
    pub layout_ids: Vec<String>,

    /// the layouts used to create pipelines
    pub(crate) layouts: Vec<Arc<wgpu::BindGroupLayout>>,
}

/// Preset rendering pipelines
pub enum Pipeline {
    /// Simple 2D colored sprite rendering pipeline
    ColoredSprite,
}

impl Pipeline {
    /// Returns a RenderPipelineConfig corresponding to the pipeline preset variant
    pub fn get(self) -> RenderPipelineConfig {
        return match self {
            Pipeline::ColoredSprite => RenderPipelineConfig {
                name: "colored-sprite".to_string(),
                shader_path: "src/graphics/shaders/shader.wgsl".to_string(),
                vert_main: "vs_main".to_string(),
                frag_main: "fs_main".to_string(),
                layout_ids: vec!["colored-sprite".to_string()],
                layouts: Vec::new()
            },
        }
    }
}

pub enum BindingLayout {
    ColoredSprite
}

impl BindingLayout {
    pub fn get(self) -> Arc<LayoutConfig> {
        return match self {
            BindingLayout::ColoredSprite => {
                let model_mat = LayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: BindingType::Uniform
                };

                let color = LayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: BindingType::Uniform
                };

                Arc::new(LayoutConfig { 
                    key: "colored-sprite".to_string(),
                    bind_group: MATERIAL_GROUP, 
                    entries: vec![model_mat, color]
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