#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{buffer::{BufferBuilder, BufferContext}, handler::ResourceBuilder, vertex::{VertexAttribute, VertexLayoutBuilder}};

/// represents vertex/index buffers created from a GeometryBuilder
pub struct GeometryBuffer {
    pub vertex_buffer: Arc<wgpu::Buffer>,
    pub index_buffer: Arc<wgpu::Buffer>,
    pub num_indices: u32,
}

/// struct for building and storing unique geometry
pub struct Geometry {
    pub id: GeometryID,
    pub builder: Arc<GeometryBuilder>
}

impl Geometry {
    /// get this geometry namespaced to it's label and layout
    pub fn get_key(&self) -> String {
        format!("{}::{}", self.id.label, self.id.vertex_layout.key_str())
    }
}

/// key for geometry with certain attributes
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct GeometryID {
    pub label: String,
    pub vertex_layout: VertexLayoutBuilder
}

/// Represents a single vertex in a mesh
pub struct Vertex {
    pub position: Option<[f32; 3]>,
    pub uv: Option<[f32; 2]>,
    pub normal: Option<[f32; 3]>
}

/// Constructs interleaved geometry data and gpu buffers.
#[derive(Clone, Debug)]
pub struct GeometryBuilder {
    label: String,
    vertex_data: Vec<u8>,
    index_data: Vec<u32>,
    vertex_layout: VertexLayoutBuilder,
}

impl GeometryBuilder {
    pub fn new(vertex_layout: VertexLayoutBuilder) -> Self {
        Self {
            label: "geometry".to_string(),
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            vertex_layout,
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

    /// Add vertices to the geometry builder in the form of a byte vector
    pub fn with_vertex_data(mut self, vertices: Vec<u8>) -> Self {
        self.vertex_data = vertices;
        self
    }

    /// Add index data to the geometry
    pub fn with_indices(mut self, indices: Vec<u32>) -> Self {
        self.index_data = indices;
        self
    }

    /// Add a single vertex to the geometry
    pub fn add_vertex(&mut self, vertex: Vertex) {
        for attr in self.vertex_layout.get_attributes() {
            match attr {
                VertexAttribute::Position => {
                    if let Some(position) = vertex.position {
                        self.vertex_data.extend_from_slice(bytemuck::cast_slice(&position));
                    }
                },
                VertexAttribute::Normal => {
                    if let Some(normal) = vertex.normal {
                        self.vertex_data.extend_from_slice(bytemuck::cast_slice(&normal));
                    }
                },
                VertexAttribute::UV => {
                    if let Some(uv) = vertex.uv {
                        self.vertex_data.extend_from_slice(bytemuck::cast_slice(&uv));
                    }
                },
                _ => {}, // skip non-vertex specific attributes
            }
        }
    }
}

impl ResourceBuilder for Arc<GeometryBuilder> {
    type Context = BufferContext;
    type Output = GeometryBuffer;

    fn build(&self, context: Arc<Self::Context>) -> Result<Self::Output, String> {
        let vertex_data: Vec<u8> = bytemuck::cast_slice(&self.vertex_data).to_vec();
        let index_data: Vec<u8> = bytemuck::cast_slice(&self.index_data).to_vec();

        let vertex_buffer = BufferBuilder::as_vertex()
            .with_label(&format!("{}_vertices", self.label))
            .with_data(vertex_data)
            .build(Arc::clone(&context))?;

        let index_buffer = BufferBuilder::as_index()
            .with_label(&format!("{}_indices", self.label))
            .with_data(index_data)
            .build(Arc::clone(&context))?;

        println!("[GeometryBuilder] Created new geometry with label '{}", self.label);

        Ok(GeometryBuffer {
            vertex_buffer,
            index_buffer,
            num_indices: self.index_data.len() as u32
        })
    }
}