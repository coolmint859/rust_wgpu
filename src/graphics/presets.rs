#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{
   handler::ResourceHandler, 
   mesh::MeshData, 
   render_pipeline::RenderPipelineBuilder, 
   texture::SamplerBuilder, 
   vertex::{PositionVertex, UV_Vertex}
};

/// Preset rendering pipelines
pub enum RenderPipeline {
    /// Simple 2D colored sprite rendering pipeline
    ColoredSprite,
    TexturedSprite,
}

impl RenderPipeline {
    /// RGet the RenderPipelineBuilder that this RenderPipeline represents
    pub fn get(self) -> RenderPipelineBuilder {
        return match self {
            RenderPipeline::ColoredSprite => {
                let path = "src/graphics/shaders/colored_sprite.wgsl";
                RenderPipelineBuilder::new::<PositionVertex>(path, 3).with_label("colored-sprite")
            }
            RenderPipeline::TexturedSprite => {
                let path = "src/graphics/shaders/textured_sprite.wgsl";
                RenderPipelineBuilder::new::<UV_Vertex>(path, 3).with_label("textured-sprite")
            }
        }
    }
}

/// Represents a sampler with a specific address and filter mode, as supported by wgpu
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum TextureSampler {
    NearestClampEdge,
    NearestClampBorder,
    NearestRepeat,
    NearestMirrorRepeat,
    LinearClampEdge,
    LinearClampBorder,
    LinearRepeat,
    LinearMirrorRepeat,
}

impl TextureSampler {
    /// Get the SamplerBuilder that this TextureSampler represents
    pub fn get(self) -> SamplerBuilder {
        match self {
            TextureSampler::NearestRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::Repeat, wgpu::FilterMode::Nearest).with_label("nearest_repeat")
            },
            TextureSampler::NearestClampEdge => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToEdge, wgpu::FilterMode::Nearest).with_label("nearest_clamp-to-edge")
            },
            TextureSampler::NearestClampBorder => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToBorder, wgpu::FilterMode::Nearest).with_label("nearest_clamp-to-border")
            },
            TextureSampler::NearestMirrorRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::MirrorRepeat, wgpu::FilterMode::Nearest).with_label("nearest_mirror_repeat")
            },
            TextureSampler::LinearRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::Repeat, wgpu::FilterMode::Linear).with_label("linear_repeat")
            },
            TextureSampler::LinearMirrorRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::MirrorRepeat, wgpu::FilterMode::Linear).with_label("linear_mirror-repeat")
            },
            TextureSampler::LinearClampEdge => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToEdge, wgpu::FilterMode::Linear).with_label("linear_clamp-to-edge")
            },
            TextureSampler::LinearClampBorder => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToBorder, wgpu::FilterMode::Linear).with_label("linear_clamp-to-border")
            }
        }
    }

    pub fn as_key(self) -> String {
        match self {
            TextureSampler::LinearClampBorder => "linear_clamp-to-border".to_string(),
            TextureSampler::LinearClampEdge => "linear_clamp-to-edge".to_string(),
            TextureSampler::LinearMirrorRepeat => "linear_mirror-repeat".to_string(),
            TextureSampler::LinearRepeat => "linear_repeat".to_string(),
            TextureSampler::NearestClampBorder => "nearest_clamp-to-border".to_string(),
            TextureSampler::NearestClampEdge => "nearest_clamp-to-edge".to_string(),
            TextureSampler::NearestRepeat => "nearest_repeat".to_string(),
            TextureSampler::NearestMirrorRepeat => "nearest_mirror-repeat".to_string(),
        }
    }
}

/// Generates and stores 2D shapes
pub struct Shape2D {
    shape_data: ResourceHandler<String, Arc<MeshData>>,
}

impl Shape2D {
    pub fn new() -> Self {
        Self { shape_data: ResourceHandler::new() }
    }

    /// Generate mesh data for a triangle
    // pub fn triangle(&mut self) -> Arc<MeshData> {
    //     let key = "triangle".to_string();

    //     return match self.shape_data.get(&key) {
    //         Some(data) => Arc::clone(data),
    //         None => {
    //             let triangle = Arc::new(gen_triangle());
    //             self.shape_data.store(&key, Arc::clone(&triangle));
    //             Arc::clone(&triangle)
    //         }
    //     }
    // }

    /// Generate mesh data for a square.
    pub fn square(&mut self) -> Arc<MeshData> {
        let key = "square".to_string();
        
        return match self.shape_data.get(&key) {
            Some(data) => Arc::clone(data),
            None => {
                let square = Arc::new(gen_square());
                self.shape_data.store(&key, Arc::clone(&square));
                Arc::clone(&square)
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
// pub fn gen_triangle() -> MeshData {
//     MeshData::new(
//         vec![
//             PositionVertex { position: [0.0, 0.5, 0.0] },
//             PositionVertex { position: [-0.5, -0.5, 0.0] },
//             PositionVertex { position: [0.5, -0.5, 0.0] },
//         ],
//         vec![0, 1, 2]
//     )
//     .with_label("triangle")
// }

/// Get raw square data
pub fn gen_square() -> MeshData  {
    MeshData::new(
        vec![
            UV_Vertex { position: [ 0.5,  0.5, 0.0], uv: [1.0, 0.0] },
            UV_Vertex { position: [-0.5,  0.5, 0.0], uv: [0.0, 0.0] },
            UV_Vertex { position: [-0.5, -0.5, 0.0], uv: [0.0, 1.0] },
            UV_Vertex { position: [ 0.5, -0.5, 0.0], uv: [1.0, 1.0] },
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