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
    Normal
}

impl VertexAttribute {
    pub fn format(self) -> wgpu::VertexFormat {
        match self {
            VertexAttribute::Position => wgpu::VertexFormat::Float32x3,
            VertexAttribute::Normal => wgpu::VertexFormat::Float32x3,
            VertexAttribute::UV => wgpu::VertexFormat::Float32x2
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct VertexLayoutBuilder {
    attributes: Vec<VertexAttribute>
}

impl VertexLayoutBuilder {
    pub fn new() -> Self {
        Self {
            attributes: Vec::new()
        }
    }

    /// create a vertex layout with a position attribute
    pub fn with_position() -> Self {
        Self {
            attributes: vec![VertexAttribute::Position]
        }
    }

    /// Add an attribute to the vertex_layout
    pub fn with_attribute(mut self, attr: VertexAttribute) -> Self {
        self.attributes.push(attr);
        self
    }

    /// Add an attribute to the vertex_layout
    pub fn add_attribute(&mut self, attr: VertexAttribute) {
        self.attributes.push(attr);
    }

    /// Get the set of vertex attributes for this vertex layout
    pub fn get_attributes(&self) -> Vec<VertexAttribute> {
        self.attributes.to_vec()
    }
}

impl ResourceBuilder for VertexLayoutBuilder {
    type Context = (); // no context needed
    type Output = wgpu::VertexBufferLayout<'static>;

    fn build(&self, _context: Arc<Self::Context>) -> Result<Self::Output, String> {
        let mut attributes = Vec::new();

        let mut offset = 0;
        for (i, attr) in self.attributes.iter().enumerate() {
            let format = attr.clone().format();
            attributes.push(wgpu::VertexAttribute {
                offset,
                shader_location: i as u32,
                format: format.clone()
            });

            offset += format.size()
        }

        Ok(wgpu::VertexBufferLayout {
            array_stride: offset,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: attributes.leak()
        })
    }
}