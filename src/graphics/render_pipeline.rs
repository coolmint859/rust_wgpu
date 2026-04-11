#![allow(dead_code)]
use std::{borrow::Cow, sync::Arc};

use crate::graphics::{
    gpu_resource::{ResourceBuilder, ResourceTemplate}, 
    vertex::Vertex, wpgu_context::RenderPipelineContext
};

/// Allows creation of pipelines from a template.
/// 
/// Also serves as the key to the corresponding concrete render pipelines
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct RenderPipelineTemplate {
    pub label: String, // marked pub to allow easy identification in debugging

    shader_path: String,
    vs_main: String,
    fs_main: String,

    vertex_stride: u64,
    vertex_attribs: Vec<wgpu::VertexAttribute>,

    layout_ids: Vec<String>,
    topology: wgpu::PrimitiveTopology,

    blend_state: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,
}

impl RenderPipelineTemplate {
    pub fn new<V: Vertex>(shader_path: &str) -> Self {
        Self {
            label: "default-pipeline".to_string(),
            shader_path: shader_path.to_string(),
            vs_main: "vs_main".to_string(),
            fs_main: "fs_main".to_string(),
            vertex_stride: std::mem::size_of::<V>() as u64,
            vertex_attribs: V::attributes(),
            layout_ids: Vec::new(),
            topology: wgpu::PrimitiveTopology::TriangleList,
            blend_state: Some(wgpu::BlendState::REPLACE),
            cull_mode: Some(wgpu::Face::Back),
        }
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
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
    pub fn with_bg_layout(mut self, id: &str) -> Self {
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
}

impl ResourceTemplate for RenderPipelineTemplate {
    type Builder = RenderPipelineBuilder;

    fn to_builder(&self) -> Self::Builder {
        RenderPipelineBuilder::from_template(self.clone())
    }
}

#[derive(Clone, Debug)]
/// Implements the builder pattern for constructing render pipelines
pub struct RenderPipelineBuilder {
    template: RenderPipelineTemplate
}

impl RenderPipelineBuilder {
    pub fn from_template(template: RenderPipelineTemplate) -> Self {
        Self { template }
    }
}

impl ResourceBuilder for RenderPipelineBuilder {
    type Output = wgpu::RenderPipeline;
    type Context = RenderPipelineContext;

    /// Construct the render pipeline with the settings provided
    fn build(&self, context: Arc<RenderPipelineContext>) -> Result<Self::Output, String> {
        if self.template.vertex_stride == 0 {
            return Err("Expected vertex layout but was builder was not configured with one.".to_string())
        }

        let shader_source = match std::fs::read_to_string(&self.template.shader_path) {
            Ok(source) => source,
            Err(e) => {
                return Err(format!("Failed to read shader file '{}': {e}", self.template.shader_path));
            }
        };

        let shader = context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(format!("{}@{}", self.template.label, self.template.shader_path).as_str()),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
        });

        let layout_refs: Vec<&wgpu::BindGroupLayout> = context.layouts
            .iter()
            .map(|arc| arc.as_ref()) // or just &**arc
            .collect();

        let pipeline_layout = context.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some(&self.template.label),
                bind_group_layouts: &layout_refs,
                immediate_size: 0,
            }
        );

        let vertex = wgpu::VertexState {
            module: &shader,
            entry_point: Some(&self.template.vs_main),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: self.template.vertex_stride,
                attributes: &self.template.vertex_attribs.as_slice(),
                step_mode: wgpu::VertexStepMode::Vertex
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        let fragment = Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(&self.template.fs_main),
            targets: &[Some(wgpu::ColorTargetState {
                format: context.format,
                blend: self.template.blend_state,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        let primitive = wgpu::PrimitiveState {
            topology: self.template.topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: self.template.cull_mode,
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

        let render_pipeline = context.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some(&self.template.label),
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

        println!("[Render Pipeline] Created new render pipeline with label '{}'", self.template.label);

        Ok(render_pipeline)
    }
}
