#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use crate::graphics::{buffer::BufferBuilder, data_utils::{DirtyVec, DynHashMap, DynVector, PackingUtils}, vertex::{NORMAL_LOC, POSITION_LOC, UV_LOC, VertexAttribute, VertexLayoutBuilder}, wpgu_context::{GeometryID, ResourceID, ResourceScope, ResourceType}};

pub const POSITION_ATTR: &str = "position";
pub const UV_ATTR: &str = "uv";
pub const NORMAL_ATTR: &str = "normal";

/// Contains the raw geometry data as provided. The hashmap serves as a generic store to make it easy to add custom data.
pub struct GeometryData {
    /// Generic label for the data
    pub label: String,
    /// A map of attribute names to their data
    pub attributes: DynHashMap,
    /// vertex indices
    pub indices: Option<Arc<Vec<u32>>>,
    /// The number of vertices on this geometry
    pub vertex_count: usize
}

impl GeometryData {
    pub fn new(vertex_count: usize) -> Self {
        Self {
            label: "geometry".to_string(),
            attributes: DynHashMap { map: HashMap::new() },
            indices: None,
            vertex_count
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// add attribute data to this geometry
    pub fn with_attr(mut self, key: &str, data: impl DynVector) -> Self {
        self.attributes.map.insert(key.to_string(), Box::new(data));
        self
    }

    /// add index data to this geometry
    pub fn with_indices(mut self, indices: Vec<u32>) -> Self {
        self.indices = Some(Arc::new(indices));
        self
    }
}

/// The signature of a geometry, including the buffer ids and their builders
pub struct GeometrySignature {
    pub ids: GeometryID,
    pub vertex_builder: BufferBuilder,
    pub index_builder: BufferBuilder,
}

pub struct Geometry {
    data: Arc<GeometryData>,
    attributes: Vec<Box<dyn VertexAttribute>>,
    packed_data: Option<Vec<u8>>,
}

impl Geometry {
    pub fn new(data: Arc<GeometryData>) -> Self {
        Self {
            data,
            attributes: Vec::new(),
            packed_data: None,
        }
    }

    /// Get the generic label for this geometry
    pub fn get_label(&self) -> String {
        self.data.label.clone()
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
            key: format!("{}::vertices", self.data.label.clone()),
            scope: ResourceScope::Entity,
            r_type: ResourceType::Vertex(self.get_layout_builder())
        };

        let index_id = ResourceID { 
            key: format!("{}::indices", self.data.label.clone()),
            scope: ResourceScope::Entity,
            r_type: ResourceType::Index
        };

        let indices = self.data.indices.as_ref().unwrap().len() as u32;

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
            None => PackingUtils::pack(self.data.vertex_count, &self.data.attributes, &self.attributes)
        };

        let index_data = match &self.data.indices {
            Some(indices) => bytemuck::cast_slice(&indices).to_vec(),
            None => panic!("[Geometry] Missing index data for shape '{}'", self.data.label)
        };

        let vertex_builder = BufferBuilder::as_vertex()
            .with_label(&format!("{}::vertices", self.data.label))
            .with_data(vertex_data);

        let index_builder = BufferBuilder::as_index()
            .with_label(&format!("{}::indices", self.data.label))
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
    fn name(&self) -> &'static str { POSITION_ATTR }
    fn location(&self) -> u32 { POSITION_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, bytes: &mut Vec<u8>) {
        if let Some(positions) = attributes.get_vec::<DirtyVec<glam::Vec3>>(POSITION_ATTR) {
            bytes.extend_from_slice(bytemuck::bytes_of(&*positions.inner[idx]));
        }
    }
}

/// A vertex attribute for uv (texture) coordinates
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct UVAttribute;

impl VertexAttribute for UVAttribute {
    fn name(&self) -> &'static str { UV_ATTR }
    fn location(&self) -> u32 { UV_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(uvs) = attributes.get_vec::<DirtyVec<glam::Vec2>>(UV_ATTR) {
            buffer.extend_from_slice(bytemuck::bytes_of(&*uvs.inner[idx]));
        }
    }
}

/// A vertex attribute for normals
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct NormalAttribute;

impl VertexAttribute for NormalAttribute {
    fn name(&self) -> &'static str { NORMAL_ATTR }
    fn location(&self) -> u32 { NORMAL_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(normals) = attributes.get_vec::<DirtyVec<glam::Vec3>>(NORMAL_ATTR) {
            buffer.extend_from_slice(bytemuck::bytes_of(&*normals.inner[idx]));
        }
    }
}