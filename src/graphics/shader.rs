use std::{borrow::Cow, sync::Arc};
use crate::graphics::{bind_group::BindGroupLayoutBuilder, entity::Entity, handler::ResourceBuilder, vertex::{VertexAttribute, VertexLayoutBuilder}};

#[derive(Clone, Debug)]
pub struct ShaderSpec {
    pub shader_builder: ShaderModuleBuilder,
    pub vs_main: String,
    pub fs_main: String,
    pub vertex_attrs: Vec<(VertexAttribute, u32)>,
    pub instance_attrs: Vec<(VertexAttribute, u32)>,
    pub bg_layouts: Vec<BindGroupLayoutBuilder>,
}

impl ShaderSpec {
    /// create the vertex layout from the provided attributes
    pub fn build_vertex_layouts(&self) -> Vec<wgpu::VertexBufferLayout<'static>> {
        let mut layouts = Vec::new();

        // vertex attributes
        if !self.vertex_attrs.is_empty() {
            let mut vertex_layout_builder = VertexLayoutBuilder::new();
            for (attr, loc) in &self.vertex_attrs {
                vertex_layout_builder.add_attribute(attr.clone(), loc.clone());
            }
            layouts.push(vertex_layout_builder.build(Arc::new(())).unwrap());
        }

        // instance attributes
        if !self.instance_attrs.is_empty() {
            let mut instance_layout_builder = VertexLayoutBuilder::new()
                .with_step_mode(wgpu::VertexStepMode::Instance);

            for (attr, loc) in &self.instance_attrs {
                instance_layout_builder.add_attribute(attr.clone(), loc.clone());
            }
            layouts.push(instance_layout_builder.build(Arc::new(())).unwrap());
        }

        layouts
    }

    /// validate that an entity can be drawn with this shader
    pub fn _validate(&self, _entity: &Entity) -> bool {
        return true;
    }

    /// Get the set of vertex attributes for this vertex layout
    pub fn get_vertex_attributes(&self) -> Vec<(VertexAttribute, u32)> {
        self.vertex_attrs.clone()
    }

    /// Get the interleaved width of the vertex data as determined from the attributes
    pub fn _vertex_stride(&self) -> usize {
        let mut stride = 0;
        for (attr, _) in &self.vertex_attrs {
            stride += attr.format().size()
        }
        stride as usize
    }
}

#[derive(Clone, Debug)]
pub struct ShaderSpecBuilder {
    pub path: String,
    pub vertex_attrs: Vec<(VertexAttribute, u32)>,
    pub instance_attrs: Vec<(VertexAttribute, u32)>,
    pub bg_layouts: Vec<BindGroupLayoutBuilder>,
}

impl ShaderSpecBuilder {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            vertex_attrs: Vec::new(),
            instance_attrs: Vec::new(),
            bg_layouts: Vec::new(),
        }
    }

    pub fn with_vertex_attribute(mut self, attr: VertexAttribute, loc: u32) -> Self {
        self.vertex_attrs.push((attr, loc));
        self
    }

    pub fn with_instance_attribute(mut self, attr: VertexAttribute, loc: u32) -> Self {
        self.instance_attrs.push((attr, loc));
        self
    }

    pub fn with_bg_layout(mut self, layout: BindGroupLayoutBuilder) -> Self {
        self.bg_layouts.push(layout);
        self
    }
}

impl ResourceBuilder for ShaderSpecBuilder {
    type Output = ShaderSpec;
    type Context = ();

    fn build(&self, _context: Arc<()>) -> Result<ShaderSpec, String> {
        let shader_source = match std::fs::read_to_string(&self.path) {
            Ok(source) => source,
            Err(e) => {
                return Err(format!("Failed to read shader file '{}': {e}", self.path));
            }
        };

        // Shader reflection done here

        Ok(ShaderSpec {
            shader_builder: ShaderModuleBuilder::new(self.path.clone(), shader_source),
            fs_main: "fs_main".to_string(),
            vs_main: "vs_main".to_string(),
            vertex_attrs: self.vertex_attrs.clone(),
            instance_attrs: self.instance_attrs.clone(),
            bg_layouts: self.bg_layouts.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ShaderModuleBuilder {
    pub path: String,
    pub source: String,
}

impl ShaderModuleBuilder {
    pub fn new(path: String, source: String) -> Self {
        Self { path, source }
    }
}

impl ResourceBuilder for ShaderModuleBuilder {
    type Context = wgpu::Device;
    type Output = Arc<wgpu::ShaderModule>;

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(format!("{}", self.path).as_str()),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&self.source)),
        });

        println!("[Shader Module] Created new shader module from path {}", self.path);

        Ok(Arc::new(shader))
    }
}