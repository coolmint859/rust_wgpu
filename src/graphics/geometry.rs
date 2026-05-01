#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{buffer::{BufferBuilder, BufferContext}, handler::ResourceBuilder, vertex::VertexAttribute};

/// represents vertex/index buffers created from a GeometryBuilder
pub struct GeometryBuffer {
    pub vertex_buffer: Arc<wgpu::Buffer>,
    pub index_buffer: Arc<wgpu::Buffer>,
    pub num_indices: u32,
}

pub struct GeometryContext {
    pub buffer_context: Arc<BufferContext>,
    pub attrs: Vec<(VertexAttribute, u32)>,
    pub stride: usize
}

/// key for geometry with certain attributes
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct GeometryID {
    pub key: String,
    pub attrs: Vec<VertexAttribute>
}

/// Represents a single vertex in a mesh
#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: Option<[f32; 3]>,
    pub uv: Option<[f32; 2]>,
    pub normal: Option<[f32; 3]>
}

/// Constructs interleaved geometry data and gpu buffers.
#[derive(Clone, Debug)]
pub struct GeometryBuilder {
    label: String,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl GeometryBuilder {
    pub fn new() -> Self {
        Self {
            label: "geometry".to_string(),
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add vertices to the geometry builder
    pub fn with_vertices(mut self, vertices: Vec<Vertex>) -> Self {
        for vertex in vertices {
            self.add_vertex(vertex);
        }
        self
    }

    /// Add index data to the geometry
    pub fn with_indices(mut self, indices: Vec<u32>) -> Self {
        self.indices = indices;
        self
    }

    /// Add a single vertex to the geometry
    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    pub fn get_label(&self) -> String {
        self.label.clone()
    }

    /// filters the geometry's vertex data into a byte vector as required by the vertex layout
    fn pack_vertices(&self, attributes: &Vec<(VertexAttribute, u32)>, stride: usize) -> Vec<u8> {
        let mut vertex_data: Vec<u8> = Vec::with_capacity(self.vertices.len() * stride);

        for vertex in &self.vertices {
            for (attr, _) in attributes {
                match attr {
                    VertexAttribute::Position => {
                        if let Some(position) = &vertex.position {
                            vertex_data.extend_from_slice(bytemuck::bytes_of(position));
                        }
                    },
                    VertexAttribute::Normal => {
                        if let Some(normal) = &vertex.normal {
                            vertex_data.extend_from_slice(bytemuck::bytes_of(normal));
                        }
                    },
                    VertexAttribute::UV => {
                        if let Some(uv) = &vertex.uv {
                            vertex_data.extend_from_slice(bytemuck::bytes_of(uv));
                        }
                    },
                    _ => {}, // skip non-vertex specific attributes
                }
            }
        }
        vertex_data
    }

    fn attribute_str(&self, attrs: &Vec<(VertexAttribute, u32)>) -> String {
        let mut attr_str = "attrs_".to_string();
        for (attr, _) in attrs {
            match attr {
                VertexAttribute::Position => attr_str += "P",
                VertexAttribute::Normal => attr_str += "N",
                VertexAttribute::UV => attr_str += "UV",
                _ => ()
            }
        }

        attr_str
    } 
}

impl ResourceBuilder for GeometryBuilder {
    type Context = GeometryContext;
    type Output = GeometryBuffer;

    fn build(&self, context: Arc<Self::Context>) -> Result<Self::Output, String> {
        let vertex_data: Vec<u8> = self.pack_vertices(&context.attrs, context.stride);
        let index_data: Vec<u8> = bytemuck::cast_slice(&self.indices).to_vec();

        let id_str = format!("{}::{}", self.label, self.attribute_str(&context.attrs));

        let vertex_buffer = BufferBuilder::as_vertex()
            .with_label(&format!("{}::vertices", id_str))
            .with_data(vertex_data)
            .build(Arc::clone(&context.buffer_context))?;

        let index_buffer = BufferBuilder::as_index()
            .with_label(&format!("{}::indices", id_str))
            .with_data(index_data)
            .build(Arc::clone(&context.buffer_context))?;

        println!("[GeometryBuilder] Created new geometry with label '{}'", self.label);

        Ok(GeometryBuffer {
            vertex_buffer,
            index_buffer,
            num_indices: self.indices.len() as u32
        })
    }
}