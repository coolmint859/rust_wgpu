#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use crate::graphics::{buffer::BufferBuilder, data_utils::{DirtyVec, DynHashMap, DynVector, PackingUtils}, vertex::{NORMAL_LOC, POSITION_LOC, UV_LOC, VertexComponent, VertexLayoutBuilder}, wpgu_context::{GeometryID, ResourceID, ResourceScope, ResourceType}};

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
    components: Vec<Box<dyn VertexComponent>>,
    packed_data: Option<Vec<u8>>,
}

impl Geometry {
    pub fn new(data: Arc<GeometryData>) -> Self {
        Self {
            data,
            components: Vec::new(),
            packed_data: None,
        }
    }

    /// Get the generic label for this geometry
    pub fn get_label(&self) -> String {
        self.data.label.clone()
    }

    /// Add a vertex component to this geometry
    pub fn with_component(mut self, component: impl VertexComponent + 'static) -> Self {
        self.add_component(component);
        self
    }

    /// Add a vertex component to this geometry
    pub fn add_component(&mut self, component: impl VertexComponent + 'static) {
        self.components.push(Box::new(component));
        self.packed_data = None; // set to None to reset data packing
    }

    /// Get the vertex layout builder defined by this Geometry
    pub fn get_layout_builder(&self) -> VertexLayoutBuilder {
        PackingUtils::layout_builder(wgpu::VertexStepMode::Vertex, &self.components)
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
            None => PackingUtils::pack(self.data.vertex_count, &self.data.attributes, &self.components)
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

/// Represents a position attribute on a vertex
/// 
/// When applied to geometry, this acts as a marker to allow the underlying buffer to store position data
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct PositionComponent;

impl VertexComponent for PositionComponent {
    fn attribute(&self) -> &'static str { POSITION_ATTR }
    fn location(&self) -> u32 { POSITION_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, bytes: &mut Vec<u8>) {
        if let Some(positions) = attributes.get_vec::<DirtyVec<glam::Vec3>>(POSITION_ATTR) {
            bytes.extend_from_slice(bytemuck::bytes_of(&*positions.inner[idx]));
        }
    }
}

/// Represents a uv attribute (texture coordinates) on a vertex
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct UVComponent;

impl VertexComponent for UVComponent {
    fn attribute(&self) -> &'static str { UV_ATTR }
    fn location(&self) -> u32 { UV_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(uvs) = attributes.get_vec::<DirtyVec<glam::Vec2>>(UV_ATTR) {
            buffer.extend_from_slice(bytemuck::bytes_of(&*uvs.inner[idx]));
        }
    }
}

/// Represents a normal attribute on a vertex
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct NormalComponent;

impl VertexComponent for NormalComponent {
    fn attribute(&self) -> &'static str { NORMAL_ATTR }
    fn location(&self) -> u32 { NORMAL_LOC }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }

    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>) {
        if let Some(normals) = attributes.get_vec::<DirtyVec<glam::Vec3>>(NORMAL_ATTR) {
            buffer.extend_from_slice(bytemuck::bytes_of(&*normals.inner[idx]));
        }
    }
}