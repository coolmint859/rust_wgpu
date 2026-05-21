#![allow(dead_code)]
use std::{ops::Range, sync::Arc};

use crate::graphics::data_utils::DynHashMap;

use super::handler::ResourceBuilder;

pub const VERTEX_LOCATIONS: Range<u32> = 0..7;
pub const INSTANCE_LOCATIONS: Range<u32> = 8..15;

/// vertex position shader location
pub const POSITION_LOC: u32 = 0;
/// vertex tex coords shader location
pub const UV_LOC: u32 = 1;
/// vertex normal tex coords shader location
pub const NORMAL_LOC: u32 = 2;

/// instance transform shader location
pub const TRANSFORM_LOC: u32 = 8;
/// instance tint shader location
pub const TINT_LOC: u32 = 12;
/// instance spritesheet texture bounds shader location
pub const UV_BOUNDS_LOC: u32 = 13;

/// represents attributes that can be packed into a vertex/instance buffer
pub trait VertexAttribute {
    /// the name of the attribute the component represents
    fn name(&self) -> &'static str;

    /// the attribute location in the shader
    fn location(&self) -> u32;

    /// The format of the attribute data in the shader
    fn format(&self) -> wgpu::VertexFormat;

    /// The number of vertex attributes the component requires (default is 1)
    fn attr_count(&self) -> u32 { 1 }

    /// Writes data at the specified index to a byte vector.
    fn write_to(&self, idx: usize, attributes: &DynHashMap, buffer: &mut Vec<u8>);
}

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

    /// create a vertex layout with a position attribute
    pub fn with_position() -> Self {
        VertexLayoutBuilder::new(wgpu::VertexStepMode::Vertex)
            .with_attribute(POSITION_LOC, wgpu::VertexFormat::Float32x4)
    }

    /// create a vertex layout with a transform attribute and instance step mode
    pub fn with_transform() -> Self {
        VertexLayoutBuilder::new(wgpu::VertexStepMode::Vertex)
            .with_attribute(TRANSFORM_LOC, wgpu::VertexFormat::Float32x4)
            .with_attribute(TRANSFORM_LOC+1, wgpu::VertexFormat::Float32x4)
            .with_attribute(TRANSFORM_LOC+2, wgpu::VertexFormat::Float32x4)
            .with_attribute(TRANSFORM_LOC+3, wgpu::VertexFormat::Float32x4)
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

    fn build(&self, _context: Arc<()>) -> Result<Self::Output, String> {
        let layout = wgpu::VertexBufferLayout {
            array_stride: self.curr_offset,
            step_mode: self.step_mode,
            attributes: self.attributes.clone().leak()
        };

        println!("[Vertex Layout] Created new vertex layout with label '{}'.", self.label);

        Ok(Arc::new(layout))
    }
}