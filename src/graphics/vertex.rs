#![allow(dead_code)]
use bytemuck;
use wgpu;

/// construct a f32 position vertex attribute
pub fn pos_attr(offset: u64, location: u32) -> wgpu::VertexAttribute {
    wgpu::VertexAttribute {
        offset,
        shader_location: location,
        format: wgpu::VertexFormat::Float32x3,
    }
}

/// construct a f32 uv vertex attribute
pub fn uv_attr(offset: u64, location: u32) -> wgpu::VertexAttribute {
    wgpu::VertexAttribute {
        offset,
        shader_location: location,
        format: wgpu::VertexFormat::Float32x2,
    }
}

/// construct a f32 normal vertex attribute
pub fn normal_attr(offset: u64, location: u32) -> wgpu::VertexAttribute {
    wgpu::VertexAttribute {
        offset,
        shader_location: location,
        format: wgpu::VertexFormat::Float32x3,
    }
}

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
        vec![pos_attr(0, 0)]
    }
}

/// A vertex with a position and uv attribute
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct UV_Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex for UV_Vertex {
    fn attributes() -> Vec<wgpu::VertexAttribute> {
        vec![pos_attr(0, 0), uv_attr(12, 1)]
    }
}


/// A vertex with a position and normal attribute
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct NormalVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex for NormalVertex {
    fn attributes() -> Vec<wgpu::VertexAttribute> {
        vec![pos_attr(0, 0), normal_attr(12, 1)]
    }
}

/// A vertex with a position, normal and uv attribute
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct UV_NormalVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex for UV_NormalVertex {
    fn attributes() -> Vec<wgpu::VertexAttribute> {
        vec![pos_attr(0, 0), normal_attr(12, 1), uv_attr(20, 2)]
    }
}
