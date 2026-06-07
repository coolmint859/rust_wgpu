#![allow(dead_code)]
use std::sync::Arc;

use glam::{Vec2, Vec3};

use crate::graphics::{buffer::BufferBuilder, data_table::{DataTable, DirtyVec}, packing_utils::PackingUtils, vertex::{NORMAL_LOC, POSITION_LOC, UV_LOC, VertexAttribute, VertexLayoutBuilder}, wpgu_context::{GeometryID, ResourceID, ResourceScope, ResourceType}};

pub const POSITION_ATTR: &str = "position";
pub const UV_ATTR: &str = "uv";
pub const NORMAL_ATTR: &str = "normal";

/// The signature of a geometry, including the buffer ids and their builders
pub struct GeometrySignature {
    pub ids: GeometryID,
    pub vertex_builder: BufferBuilder,
    pub index_builder: BufferBuilder,
}

pub struct GeometryData {
    pub vertices: DataTable,
    pub indices: Vec<u32>,
    pub vertex_count: usize,
}

pub struct Geometry {
    geometry_data: Arc<GeometryData>,
    attributes: Vec<Box<dyn VertexAttribute>>,
    packed_data: Option<Vec<u8>>,
}

impl Geometry {
    pub fn new(geometry_data: Arc<GeometryData>) -> Self {
        Self {
            geometry_data,
            attributes: Vec::new(),
            packed_data: None,
        }
    }

    /// Get the generic label for this geometry
    pub fn get_label(&self) -> String {
        self.geometry_data.vertices.label.clone()
    }

    /// Add a vertex component to this geometry
    pub fn with_attribute(mut self, attribute: impl VertexAttribute + 'static) -> Self {
        self.add_attribute(attribute);
        self
    }

    /// Add a vertex component to this geometry
    pub fn add_attribute(&mut self, attribute: impl VertexAttribute + 'static) {
        self.attributes.push(Box::new(attribute));
        self.packed_data = None; // set to None to reset data packing
    }

    /// Get the vertex layout builder defined by this Geometry
    pub fn get_layout_builder(&self) -> VertexLayoutBuilder {
        PackingUtils::layout_builder(wgpu::VertexStepMode::Vertex, &self.attributes)
    }

    /// Get the ids to the buffers associated with this geometry
    pub fn get_ids(&self) -> GeometryID {
        let vertex_id = ResourceID { 
            key: format!("{}::vertices", self.get_label()),
            scope: ResourceScope::Primitive,
            r_type: ResourceType::Vertex(self.get_layout_builder())
        };

        let index_id = ResourceID { 
            key: format!("{}::indices", self.get_label()),
            scope: ResourceScope::Primitive,
            r_type: ResourceType::Index
        };

        let indices = self.geometry_data.indices.len() as u32;

        GeometryID { vertex_id, index_id, indices }
    }

    /// Get the signature of this geometry.
    /// Returns the ids to the buffers on the gpu and the builders used construct the buffers.
    /// 
    /// # Panics
    /// 
    /// If the index data is missing from the provided GeometryData
    pub fn get_signature(&self) -> GeometrySignature {
        let vertex_data = match &self.packed_data {
            Some(data) => data.to_vec(),
            None => PackingUtils::pack(self.geometry_data.vertex_count, &self.geometry_data.vertices, &self.attributes)
        };
        let index_data = bytemuck::cast_slice(&self.geometry_data.indices).to_vec();

        let vertex_builder = BufferBuilder::as_vertex()
            .with_label(&format!("{}::vertices", self.get_label()))
            .with_data(vertex_data);

        let index_builder = BufferBuilder::as_index()
            .with_label(&format!("{}::indices", self.get_label()))
            .with_data(index_data);

        GeometrySignature {
            ids: self.get_ids(),
            vertex_builder,
            index_builder
        }
    }
}

/// A vertex attribute for positions
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct PositionAttribute;

impl VertexAttribute for PositionAttribute {
    fn name(&self) -> String { POSITION_ATTR.to_string() }
    fn location(&self) -> u32 { POSITION_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(position) = vertices.get_property::<DirtyVec<Vec3>>(&self.name())
            .and_then(|positions| positions.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(position.as_ref()));
        }
    }
}

/// A vertex attribute for uv (texture) coordinates
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct UVAttribute;

impl VertexAttribute for UVAttribute {
    fn name(&self) -> String { UV_ATTR.to_string() }
    fn location(&self) -> u32 { UV_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(uv) = vertices.get_property::<DirtyVec<Vec2>>(&self.name())
            .and_then(|uvs| uvs.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(uv.as_ref()));
        }
    }
}

/// A vertex attribute for normals
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct NormalAttribute;

impl VertexAttribute for NormalAttribute {
    fn name(&self) -> String { NORMAL_ATTR.to_string() }
    fn location(&self) -> u32 { NORMAL_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(normal) = vertices.get_property::<DirtyVec<Vec3>>(&self.name())
            .and_then(|normals| normals.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(normal.as_ref()));
        }
    }
}