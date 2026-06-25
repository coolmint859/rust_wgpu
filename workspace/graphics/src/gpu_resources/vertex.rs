#![allow(dead_code)]
use std::{ops::Deref, sync::{Arc, atomic::AtomicBool}};

use crate::{
    data_table::{DataTable, DirtyVec}, 
    handler::ResourceBuilder, 
    transform::Transform, 
    wpgu_context::ResourceUpdate
};

/// The base shader location for vertex attributes
pub const VERTEX_BASE_LOC: u32 = 0;
/// The base shader location for instance attributes
pub const INSTANCE_BASE_LOC: u32 = 6;

/// Provides handles for common vertex/instance attributes
pub mod attr {
    /// Handle for position attributes (vertex)
    pub const POSITION: &'static str = "position";
    /// Handle for normal attributes (vertex)
    pub const NORMAL: &'static str = "normal";
    /// Handle for UV texture coordinate attributes (vertex)
    pub const UV_COORDS: &'static str = "uvs";
    /// Handle for Transform attributes (instance)
    pub const TRANSFORM: &'static str = "transform";
    /// Handle for uv boundaries attributes (instance)
    pub const UV_BOUNDS: &'static str = "uv_bounds";
    /// Handle for tint color attributes (instance)
    pub const TINT_COLOR: &'static str = "color_tint";
    /// Handle for text color attributes (instance)
    pub const TEXT_COLOR: &'static str = "text_color";
    /// Handle for text outline color attributes (instance)
    pub const OUTLINE_COLOR: &'static str = "outline_color";
    /// Handle for text settings attributes (instance)
    pub const SETTINGS: &'static str = "text_settings";
}

/// Represents a wgpu VertexLayout with automatic shader location mapping and data packing.
#[derive(Debug)]
pub struct VertexAttributeLayout {
    pub attributes: Vec<Box<dyn VertexAttribute>>,
    pub step_mode: wgpu::VertexStepMode,
    pub stride: usize
}

impl VertexAttributeLayout {
    /// Create a new VertexAttributeLayout with a Vertex step mode.
    pub fn as_vertex() -> Self {
        Self { 
            attributes: Vec::new(),
            step_mode: wgpu::VertexStepMode::Vertex,
            stride: 0,
        }
    }

    /// Create a new VertexAttributeLayout with an Instance step mode.
    pub fn as_instance() -> Self {
        Self { 
            attributes: Vec::new(),
            step_mode: wgpu::VertexStepMode::Instance,
            stride: 0,
        }
    }

    /// Add an attribute to this layout
    pub fn with_attribute(mut self, attr: impl VertexAttribute + 'static) -> Self {
        self.add_attribute(attr);
        self
    }

    /// Add an attribute to this layout
    pub fn add_attribute(&mut self, attr: impl VertexAttribute + 'static) {
        self.stride += (attr.format().size() as u32 * attr.count()) as usize; 
        self.attributes.push(Box::new(attr));
    }

    /// Get the stride of the attributes of this layout
    pub fn stride(&self) -> usize {
        self.stride
    }

    /// Get the wgpu VertexLayout builder from this layout
    /// 
    /// Locations are calculated based on attribute insertion order and the provided step mode
    /// * 'label' - a label to attach to the builder for gpu profiling
    pub fn get_builder(&self, label: &str) -> VertexLayoutBuilder {
        let base_loc: u32 = match self.step_mode {
            wgpu::VertexStepMode::Vertex => VERTEX_BASE_LOC,
            wgpu::VertexStepMode::Instance => INSTANCE_BASE_LOC
        };

        let mut builder = VertexLayoutBuilder::new(self.step_mode)
            .with_label(label);

        let mut location = base_loc;
        for attr in &self.attributes {
            for _ in 0..attr.count() {
                builder.add_attribute(location, attr.format());
                location += 1;
            }
        }

        builder
    }

    /// Packs all attribute data into a byte vector, filtering by components
    /// 
    /// * count: **usize** - the number of instances to pack
    /// * attr_data: **&DataTable** - a reference to the attribute data table
    pub fn pack(&self, count: usize, attr_data: &DataTable) -> Vec<u8> {
        let mut packed = Vec::new();

        for i in 0..count {
            for comp in self.attributes.iter() {
                comp.write_to(i, attr_data, &mut packed);
            }
        }

        packed
    }

    /// Get the updated attribute data as a vector of resource updates.
    /// 
    /// * count: **usize** - the number of instances to check for updates
    /// * attr_data: **&DataTable** - a reference to the attribute data table
    pub fn get_updated(&self, count: usize, attr_data: &DataTable) -> Vec<ResourceUpdate> {
        let mut updates: Vec<ResourceUpdate> = Vec::new();

        let mut i = 0;
        while i < count {
            if !self.is_instance_dirty(i, attr_data) {
                i+=1;
                continue;
            }

            let start_idx = i; // save for the offset calculation
            let mut data_span: Vec<u8> = Vec::new();

            while i < count && self.is_instance_dirty(i, attr_data) {
                for attr in &self.attributes {
                    attr.write_to(i, attr_data, &mut data_span);
                }

                self.mark_instance_clean(i, attr_data);
                i+=1;
            }

            updates.push(ResourceUpdate { 
                data: data_span, 
                offset: (start_idx * self.stride()) as u64
            });
        }

        updates
    }

    /// check if an instance's attribute data is dirty (changed this frame)
    fn is_instance_dirty(&self, idx: usize, attr_data: &DataTable) -> bool {
        self.attributes.iter().any(|attr| {
            attr_data.properties.get(&attr.name())
                .map_or(false, |attr| {
                    if let Some(dirty_tracker) = attr.as_dirty_tracker() {
                        return dirty_tracker.is_dirty(idx)
                    }
                    return false;
                })
        })
    }

    /// mark an instance's attribute data as clean
    fn mark_instance_clean(&self, idx: usize, attr_data: &DataTable) {
        attr_data.properties.iter().for_each(|(_, attr)| {
            if let Some(attr_dirty) = attr.as_dirty_tracker() {
                attr_dirty.mark_clean(idx);
            }
        });
    }
}

/// Generates a wgpu Vertex Layout
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct VertexLayoutBuilder {
    label: String,
    attributes: Vec<wgpu::VertexAttribute>,
    curr_offset: u64,
    step_mode: wgpu::VertexStepMode
}

impl VertexLayoutBuilder {
    pub fn new(step_mode: wgpu::VertexStepMode) -> Self {
        Self {
            label: "vtl_builder".to_string(),
            attributes: Vec::new(),
            curr_offset: 0,
            step_mode
        }
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Add an attribute to the vertex_layout
    pub fn with_attribute(mut self, location: u32, format: wgpu::VertexFormat) -> Self {
        self.add_attribute(location, format);
        self
    }

    /// Add an attribute to the vertex_layout
    pub fn add_attribute(&mut self, location: u32, format: wgpu::VertexFormat) {
        self.attributes.push(wgpu::VertexAttribute {
            shader_location: location,
            format,
            offset: self.curr_offset
        });
    
        self.curr_offset += format.size();
    }
}

impl ResourceBuilder for VertexLayoutBuilder {
    type Context = ();
    type Output = Arc<wgpu::VertexBufferLayout<'static>>;

    fn build(&self, _context: Arc<()>, _cancel_flag: Arc<AtomicBool>) -> Result<Self::Output, String> {
        let layout = wgpu::VertexBufferLayout {
            array_stride: self.curr_offset,
            step_mode: self.step_mode,
            attributes: self.attributes.clone().leak()
        };

        println!("[Vertex Layout] Created new vertex layout with label '{}'.", self.label);

        Ok(Arc::new(layout))
    }
}

/// Represents attributes that can be packed into a vertex/instance buffer
pub trait VertexAttribute: std::fmt::Debug {
    /// the name of the attribute
    fn name(&self) -> String;
    /// The format of the attribute in the shader
    fn format(&self) -> wgpu::VertexFormat;
    /// The number of vertex attributes required (default is 1)
    fn count(&self) -> u32 { 1 }
    /// Writes data at the specified index to a byte vector.
    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>);
}

/// A Vertex Attribute for scalar values
#[derive(Clone, Debug)]
pub struct ScalarAttribute(pub &'static str);

impl VertexAttribute for ScalarAttribute {
    fn name(&self) -> String { self.0.to_string() }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32 }

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(scalar) = vertices.get_property::<DirtyVec<f32>>(&self.0)
            .and_then(|scalars| scalars.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(scalar.deref())); // call deref manually to get the inner scalar value
        }
    }
}

/// A Vertex Attribute for glam::Vec2 values
#[derive(Clone, Debug)]
pub struct Vec2Attribute(pub &'static str);

impl VertexAttribute for Vec2Attribute {
    fn name(&self) -> String { self.0.to_string() }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x2 }

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(vec2) = vertices.get_property::<DirtyVec<glam::Vec2>>(&self.0)
            .and_then(|vec2s| vec2s.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(vec2.as_ref()));
        }
    }
}

/// A Vertex Attribute for glam::Vec3 values
#[derive(Clone, Debug)]
pub struct Vec3Attribute(pub &'static str);

impl VertexAttribute for Vec3Attribute {
    fn name(&self) -> String { self.0.to_string() }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x3 }

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(vec3) = vertices.get_property::<DirtyVec<glam::Vec3>>(&self.0)
            .and_then(|vec3s| vec3s.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(vec3.as_ref()));
        }
    }
}

/// A Vertex Attribute for glam::Vec4 values
#[derive(Clone, Debug)]
pub struct Vec4Attribute(pub &'static str);

impl VertexAttribute for Vec4Attribute {
    fn name(&self) -> String { self.0.to_string() }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(vec4) = vertices.get_property::<DirtyVec<glam::Vec4>>(&self.0)
            .and_then(|vec4s| vec4s.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(vec4.as_ref()));
        }
    }
}

/// A Vertex Attribute for Transform values
#[derive(Clone, Debug)]
pub struct TransformAttribute;

impl VertexAttribute for TransformAttribute {
    fn name(&self) -> String { attr::TRANSFORM.to_string() }
    fn format(&self) -> wgpu::VertexFormat { wgpu::VertexFormat::Float32x4 }
    fn count(&self) -> u32 { 4 } // transforms take up 4 attribute locations

    fn write_to(&self, idx: usize, vertices: &DataTable, buffer: &mut Vec<u8>) {
        if let Some(transform) = vertices.get_property::<DirtyVec<Transform>>(&self.name())
            .and_then(|transforms| transforms.get(idx)) 
        {
            buffer.extend_from_slice(bytemuck::bytes_of(&transform.to_updated()));
        }
    }
}