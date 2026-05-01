#![allow(dead_code)]
use std::sync::Arc;

use super::handler::ResourceBuilder;

/// Vertex Attributes
#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub enum VertexAttribute {
    /// A position attribute
    Position,
    /// A UV attribute
    UV,
    /// A normal attribute
    Normal,
    /// A transform matrix attribute (creates 4 in total, 1 per column)
    Transform
}

impl VertexAttribute {
    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            VertexAttribute::Position => wgpu::VertexFormat::Float32x3,
            VertexAttribute::Normal => wgpu::VertexFormat::Float32x3,
            VertexAttribute::UV => wgpu::VertexFormat::Float32x2,
            VertexAttribute::Transform => wgpu::VertexFormat::Float32x4
        }
    }

    /// Returns true if this attribute is a transform matrix, false otherwise
    pub fn is_matrix(&self) -> bool {
        match self {
            VertexAttribute::Transform => true,
            _ => false
        }
    }

    /// Get a string representation of this attribute
    pub fn as_str(&self) -> String {
        match self {
            VertexAttribute::Position => "pos".to_string(),
            VertexAttribute::Normal => "norm".to_string(),
            VertexAttribute::UV => "uv".to_string(),
            VertexAttribute::Transform => "tsfm".to_string(),
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct VertexLayoutBuilder {
    attributes: Vec<(VertexAttribute, u32)>,
    step_mode: wgpu::VertexStepMode
}

impl VertexLayoutBuilder {
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
            step_mode: wgpu::VertexStepMode::Vertex,
        }
    }

    /// create a vertex layout with a position attribute
    pub fn with_position(loc: u32) -> Self {
        VertexLayoutBuilder::new().with_attribute(VertexAttribute::Position, loc)
    }

    /// create a vertex layout with a transform attribute and instance step mode
    pub fn with_transform(loc: u32) -> Self {
        VertexLayoutBuilder::new()
            .with_step_mode(wgpu::VertexStepMode::Instance)
            .with_attribute(VertexAttribute::Transform, loc)
    }

    /// Set the step mode for the builder to construct the vertex layout with
    pub fn with_step_mode(mut self, step_mode: wgpu::VertexStepMode) -> Self {
        self.step_mode = step_mode;
        self
    }

    /// Add an attribute to the vertex_layout
    pub fn with_attribute(mut self, attr: VertexAttribute, location: u32) -> Self {
        self.attributes.push((attr, location));
        self
    }

    /// Add an attribute to the vertex_layout
    pub fn add_attribute(&mut self, attr: VertexAttribute, location: u32) {
        self.attributes.push((attr, location));
    }

    /// Get a string representation of this vertex layout
    pub fn key_str(&self) -> String{
        let mut key = format!("{:?}_", self.step_mode);
        for (attr, _) in &self.attributes {
            key.extend(attr.as_str().chars());
        }

        key
    }
}

impl ResourceBuilder for VertexLayoutBuilder {
    type Context = ();
    type Output = wgpu::VertexBufferLayout<'static>;

    fn build(&self, _context: Arc<()>) -> Result<Self::Output, String> {
        let mut attributes = Vec::new();

        let mut offset = 0;
        for (attr, loc) in &self.attributes {
            let format = attr.format();
            let locations_needed = if attr.is_matrix() { 4 } else { 1 };
            
            let mut next_loc = *loc;
            for _ in 0..locations_needed {
                attributes.push(wgpu::VertexAttribute {
                    offset,
                    shader_location: next_loc,
                    format,
                });

                offset += format.size();
                next_loc += 1;
            }
        }

        let layout = wgpu::VertexBufferLayout {
            array_stride: offset,
            step_mode: self.step_mode,
            attributes: attributes.leak()
        };

        Ok(layout)
    }
}