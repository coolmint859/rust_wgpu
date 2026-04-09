use bytemuck;
use wgpu;

pub trait Vertex: Copy + Clone + bytemuck::Zeroable + bytemuck::Pod {
    fn attributes() -> Vec<wgpu::VertexAttribute>;
}

/// A vertex with only a position attribute
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct PositionVertex {
    pub position: [f32; 3],
}

impl Vertex for PositionVertex {
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
