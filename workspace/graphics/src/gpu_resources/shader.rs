use std::{borrow::Cow, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use crate::{
    bind_group::BindGroupLayoutBuilder, 
    handler::ResourceBuilder, 
    vertex::VertexLayoutBuilder
};

#[derive(Clone, Debug)]
pub struct ShaderSpec {
    pub shader_builder: ShaderModuleBuilder,
    pub vs_main: String,
    pub fs_main: String,
    pub vt_layouts: Vec<VertexLayoutBuilder>,
    pub bg_layouts: Vec<BindGroupLayoutBuilder>,
}

#[derive(Clone, Debug)]
pub struct ShaderSpecBuilder {
    pub path: String,
    pub vt_layouts: Vec<VertexLayoutBuilder>,
    pub bg_layouts: Vec<BindGroupLayoutBuilder>,
}

impl ShaderSpecBuilder {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            vt_layouts: Vec::new(),
            bg_layouts: Vec::new(),
        }
    }

    /// Add a vertex layout builder to this shader spec
    pub fn with_vt_layout(mut self, layout: VertexLayoutBuilder) -> Self {
        self.add_vt_layout(layout);
        self
    }

    /// Add a vertex layout builder to this shader spec
    pub fn add_vt_layout(&mut self, layout: VertexLayoutBuilder) {
        self.vt_layouts.push(layout);
    }

    /// Add a bind group layout builder to this shader spec
    pub fn with_bg_layout(mut self, layout: BindGroupLayoutBuilder) -> Self {
        self.add_bg_layout(layout);
        self
    }

    /// Add a bind group layout builder to this shader spec
    pub fn add_bg_layout(&mut self, layout: BindGroupLayoutBuilder) {
        self.bg_layouts.push(layout);
    }
}

impl ResourceBuilder for ShaderSpecBuilder {
    type Output = ShaderSpec;
    type Context = ();

    fn build(&self, _context: Arc<()>, cancel_flag: Arc<AtomicBool>) -> Result<ShaderSpec, String> {
        let shader_source = match std::fs::read_to_string(&self.path) {
            Ok(source) => source,
            Err(e) => {
                return Err(format!("Failed to read shader file '{}': {e}", self.path));
            }
        };

        if cancel_flag.load(Ordering::Relaxed) {
            return Err(format!("[ShaderSpec] Main thread cancelled execution for shader spec with path {}", self.path));
        }

        // Shader reflection done here

        Ok(ShaderSpec {
            shader_builder: ShaderModuleBuilder { path: self.path.clone(), source: shader_source },
            fs_main: "fs_main".to_string(),
            vs_main: "vs_main".to_string(),
            vt_layouts: self.vt_layouts.clone(),
            bg_layouts: self.bg_layouts.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ShaderModuleBuilder {
    pub path: String,
    pub source: String,
}

impl ResourceBuilder for ShaderModuleBuilder {
    type Context = wgpu::Device;
    type Output = Arc<wgpu::ShaderModule>;

    fn build(&self, device: Arc<wgpu::Device>, _cancel_flag: Arc<AtomicBool>) -> Result<Self::Output, String> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(format!("{}", self.path).as_str()),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&self.source)),
        });

        println!("[Shader Module] Created new shader module from path {}", self.path);

        Ok(Arc::new(shader))
    }
}