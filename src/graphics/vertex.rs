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
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct VertexLayoutBuilder {
    attributes: Vec<VertexAttribute>,
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
    pub fn with_position() -> Self {
        VertexLayoutBuilder::new().with_attribute(VertexAttribute::Position)
    }

    /// create a vertex layout with a transform attribute and instance step mode
    pub fn with_transform() -> Self {
        VertexLayoutBuilder::new()
            .with_step_mode(wgpu::VertexStepMode::Instance)
            .with_attribute(VertexAttribute::Transform)
    }

    /// Set the step mode for the builder to construct the vertex layout with
    pub fn with_step_mode(mut self, step_mode: wgpu::VertexStepMode) -> Self {
        self.step_mode = step_mode;
        self
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
    type Context = u32;
    type Output = (u32, wgpu::VertexBufferLayout<'static>);

    fn build(&self, start_location: Arc<u32>) -> Result<Self::Output, String> {
        let mut attributes = Vec::new();
        let mut current_location = start_location.as_ref().clone();

        let mut offset = 0;
        for attr in &self.attributes {
            let format = attr.format();
            let locations_needed = if attr.is_matrix() { 4 } else { 1 };
            
            for _ in 0..locations_needed {
                attributes.push(wgpu::VertexAttribute {
                    offset,
                    shader_location: current_location,
                    format,
                });

                offset += format.size();
                current_location += 1;
            }
        }

        let layout = wgpu::VertexBufferLayout {
            array_stride: offset,
            step_mode: self.step_mode,
            attributes: attributes.leak()
        };

        Ok((current_location, layout))
    }
}