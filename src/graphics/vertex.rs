use bytemuck;
use wgpu;

use super::traits::VertexTrait;

/// represents a single vertex on a mesh.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl VertexTrait for Vertex {
    fn attributes() -> Vec<wgpu::VertexAttribute> {
        vec![
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
        ]
    }
}
