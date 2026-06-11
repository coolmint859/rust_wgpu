#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{buffer::BufferBuilder, data_table::DataTable, vertex::{VertexAttributeLayout, VertexAttribute, VertexLayoutBuilder}, wpgu_context::{GeometryID, ResourceID, ResourceScope, ResourceType}};

/// The signature of a geometry, including the buffer ids and their builders
pub struct GeometrySignature {
    pub ids: GeometryID,
    pub vertex_builder: BufferBuilder,
    pub index_builder: BufferBuilder,
}

pub struct GeometryData {
    pub label: String,
    pub vertices: DataTable,
    pub indices: Vec<u32>,
    pub vertex_count: usize,
}

pub struct Geometry {
    geometry_data: Arc<GeometryData>,
    attributes: VertexAttributeLayout,
    packed_data: Option<Vec<u8>>,
}

impl Geometry {
    pub fn new(geometry_data: Arc<GeometryData>) -> Self {
        Self {
            geometry_data,
            attributes: VertexAttributeLayout::as_vertex(),
            packed_data: None,
        }
    }

    /// Get the generic label for this geometry
    pub fn get_label(&self) -> String {
        self.geometry_data.label.clone()
    }

    /// Add a vertex component to this geometry
    pub fn with_attribute(mut self, attribute: impl VertexAttribute + 'static) -> Self {
        self.add_attribute(attribute);
        self
    }

    /// Add a vertex component to this geometry
    pub fn add_attribute(&mut self, attribute: impl VertexAttribute + 'static) {
        self.attributes.add_attribute(attribute);
        self.packed_data = None; // set to None to reset data packing
    }

    /// Get the vertex layout builder defined by this Geometry
    pub fn get_layout_builder(&self) -> VertexLayoutBuilder {
        self.attributes.get_builder(&self.geometry_data.label)
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
            None => self.attributes.pack(self.geometry_data.vertex_count, &self.geometry_data.vertices)
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
