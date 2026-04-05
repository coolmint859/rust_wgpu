#![allow(dead_code)]
use std::{borrow::Cow, sync::Arc};

use crate::graphics::{
    gpu_resource::ResourceBuilder, 
    traits::VertexTrait
};

#[derive(Clone, Debug)]
/// Implements the builder pattern for constructing render pipelines
pub struct RenderPipelineBuilder {
    label: String,
    key: String,

    shader_path: String,
    vs_main: String,
    fs_main: String,

    vertex_stride: u64,
    vertex_attribs: Vec<wgpu::VertexAttribute>,

    layout_ids: Vec<String>,
    bg_layouts: Vec<Arc<wgpu::BindGroupLayout>>,
    topology: wgpu::PrimitiveTopology,

    blend_state: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,

    format: Option<wgpu::TextureFormat>
}

impl RenderPipelineBuilder {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            label: "default".to_string(),
            shader_path: "NOT SET".to_string(),
            vs_main: "vs_main".to_string(),
            fs_main: "fs_main".to_string(),
            vertex_stride: 0,
            vertex_attribs: Vec::new(),
            layout_ids: Vec::new(),
            bg_layouts: Vec::new(),
            topology: wgpu::PrimitiveTopology::TriangleList,
            blend_state: Some(wgpu::BlendState::REPLACE),
            cull_mode: Some(wgpu::Face::Back),
            format: None,
        }
    }

    /// Set the path to the shader file for which to create the shader module with
    pub fn with_shader(mut self, shader_path: &str) -> Self {
        self.shader_path = shader_path.to_string();
        self
    }

    /// Set the label of the pipeline descriptors for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Set the expected vertex layout
    pub fn with_vertex_layout<V: VertexTrait>(mut self) -> Self {
        self.vertex_stride = std::mem::size_of::<V>() as u64;
        self.vertex_attribs = V::attributes();
        self
    }

    /// Set the vertex stage entry function name
    pub fn with_vertex_entry(mut self, entry: &str) -> Self {
        self.vs_main = entry.to_string();
        self
    }

    /// Set the fragment stage entry function name
    pub fn with_fragment_entry(mut self, entry: &str) -> Self {
        self.fs_main = entry.to_string();
        self
    }

    /// Set the primitive topology enum variant
    pub fn with_topology(mut self, top: wgpu::PrimitiveTopology) -> Self {
        self.topology = top;
        self
    }

    /// Add a bind group layout to the render pipeline
    pub fn with_layout(mut self, id: &str) -> Self {
        self.layout_ids.push(id.to_string());
        self
    }

    /// Set the blend state to include alpha
    pub fn with_alpha_blending(mut self) -> Self {
        self.blend_state = Some(wgpu::BlendState::ALPHA_BLENDING);
        self
    }

    /// Set the blend state to a custom mode
    pub fn with_custom_blending(mut self, blend: wgpu::BlendState) -> Self {
        self.blend_state = Some(blend);
        self
    }

    /// Set the culling mask to cull the front faces of triangles
    pub fn with_front_culling(mut self) -> Self {
        self.cull_mode = Some(wgpu::Face::Front);
        self
    }

    /// Have no culling mask (All triangle faces are rendered)
    pub fn with_no_culling(mut self) -> Self {
        self.cull_mode = None;
        self
    }

    /// Get the current set layout ids
    pub(crate) fn get_layout_ids(&self) -> Vec<String> {
        self.layout_ids.clone()
    }

    /// populate the layouts from the layout ids
    pub(crate) fn populate_bg_layouts(&mut self, layouts: Vec<Arc<wgpu::BindGroupLayout>>) {
        self.bg_layouts = layouts;
    }

    pub(crate) fn set_format(&mut self, format: wgpu::TextureFormat) {
        self.format = Some(format);
    }
}

impl ResourceBuilder for RenderPipelineBuilder {
    type Key = String;
    type Output = wgpu::RenderPipeline;

    fn get_key(&self) -> Self::Key {
        self.key.clone()
    }

    /// Construct the render pipeline with the settings provided
    fn build(&self, device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        if self.shader_path == "NOT SET" {
            return Err("The shader path has not been set yet.".to_string())
        }

        if self.vertex_stride == 0 {
            return Err("Expected vertex layout but was builder was not configured with one.".to_string())
        }

        let shader_source = match std::fs::read_to_string(&self.shader_path) {
            Ok(source) => source,
            Err(e) => {
                return Err(format!("Failed to read shader file '{}': {e}", self.shader_path));
            }
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(format!("{}@{}", self.label, self.shader_path).as_str()),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
        });

        let layout_refs: Vec<&wgpu::BindGroupLayout> = self.bg_layouts
            .iter()
            .map(|arc| arc.as_ref()) // or just &**arc
            .collect();

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some(&self.label),
                bind_group_layouts: &layout_refs,
                immediate_size: 0,
            }
        );

        let vertex = wgpu::VertexState {
            module: &shader,
            entry_point: Some(&self.vs_main),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: self.vertex_stride,
                attributes: &self.vertex_attribs.as_slice(),
                step_mode: wgpu::VertexStepMode::Vertex
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        let fragment = Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(&self.fs_main),
            targets: &[Some(wgpu::ColorTargetState {
                format: self.format.unwrap(),
                blend: self.blend_state,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        let primitive = wgpu::PrimitiveState {
            topology: self.topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: self.cull_mode,
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        };

        let multisample =  wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some(&self.label),
                layout: Some(&pipeline_layout),
                vertex,
                fragment,
                primitive,
                depth_stencil: None,
                multisample,
                multiview_mask: None,
                cache: None,
            }
        );

        println!("Created pipeline '{}'", self.label);

        Ok(render_pipeline)
    }
}
