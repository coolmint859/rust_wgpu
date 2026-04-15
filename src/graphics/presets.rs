#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{
    bind_group::{BindGroupLayoutBuilder, LayoutVisibility}, handler::ResourceHandler, mesh::MeshData, render_pipeline::RenderPipelineBuilder, texture::SamplerBuilder, vertex::{PositionVertex, UV_Vertex}
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
                
                RenderPipelineBuilder::new::<PositionVertex>(path)
                    .with_label("colored-sprite")
                    .with_bg_layout(BindingLayout::Camera2D.get())
                    .with_bg_layout(BindingLayout::ColoredSprite.get())
                    .with_bg_layout(BindingLayout::Instance.get())

            }
            RenderPipeline::TexturedSprite => {
                let path = "src/graphics/shaders/textured_sprite.wgsl";
                
                RenderPipelineBuilder::new::<UV_Vertex>(path)
                    .with_label("textured-sprite")
                    .with_bg_layout(BindingLayout::Camera2D.get())
                    .with_bg_layout(BindingLayout::TexturedSprite.get())
                    .with_bg_layout(BindingLayout::Instance.get())
            }
        }
    }
}

pub enum BindingLayout {
    Instance,
    ColoredSprite,
    TexturedSprite,
    Camera2D,
}

impl BindingLayout {
    /// Get the BindGroupLayoutBuilder that this BindingLayout represents
    pub fn get(self) -> BindGroupLayoutBuilder {
        return match self {
            BindingLayout::Instance => {
                BindGroupLayoutBuilder::new()
                    .with_label("instance")
                    .with_uniform_entry(LayoutVisibility::Vertex)
            }
            BindingLayout::ColoredSprite => {
                BindGroupLayoutBuilder::new()
                    .with_label("colored-sprite")
                    .with_uniform_entry(LayoutVisibility::Fragment)
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
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::Repeat, 
                    filter: wgpu::FilterMode::Nearest 
                }
            },
            TextureSampler::NearestClampEdge => {
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::ClampToEdge, 
                    filter: wgpu::FilterMode::Nearest 
                }
            },
            TextureSampler::NearestClampBorder => {
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::ClampToBorder, 
                    filter: wgpu::FilterMode::Nearest 
                }
            },
            TextureSampler::NearestMirrorRepeat => {
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::MirrorRepeat, 
                    filter: wgpu::FilterMode::Nearest 
                }
            },
            TextureSampler::LinearRepeat => {
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::Repeat, 
                    filter: wgpu::FilterMode::Linear 
                }
            },
            TextureSampler::LinearMirrorRepeat => {
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::MirrorRepeat, 
                    filter: wgpu::FilterMode::Linear 
                }
            },
            TextureSampler::LinearClampEdge => {
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::ClampToEdge, 
                    filter: wgpu::FilterMode::Linear 
                }
            },
            TextureSampler::LinearClampBorder => {
                SamplerBuilder { 
                    address_mode: wgpu::AddressMode::ClampToBorder, 
                    filter: wgpu::FilterMode::Linear 
                }
            }
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
    pub fn triangle(&mut self) -> Arc<MeshData> {
        let key = "triangle".to_string();

        return match self.shape_data.get(&key) {
            Some(data) => Arc::clone(data),
            None => {
                let triangle = Arc::new(gen_triangle());
                self.shape_data.store(&key, Arc::clone(&triangle));
                Arc::clone(&triangle)
            }
        }
    }

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