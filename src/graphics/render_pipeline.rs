#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::shader::ShaderSpec;

use super::{
    handler::ResourceBuilder,
};

/// Houses the environment needed to construct rendering pipelines
#[derive(Clone)]
pub struct RenderPipelineContext {
    pub device: Arc<wgpu::Device>,
    pub format: wgpu::TextureFormat,
    pub shader: Arc<wgpu::ShaderModule>,
    pub shader_spec: ShaderSpec,
    pub bg_layouts: Vec<Arc<wgpu::BindGroupLayout>>
}

/// Allows creation of pipelines from a template.
/// 
/// Also serves as the key to the corresponding concrete render pipelines
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct RenderPipelineBuilder {
    pub label: String, // marked pub to allow easy identification in debugging,

    topology: wgpu::PrimitiveTopology,
    blend_state: Option<wgpu::BlendState>,
    cull_mode: Option<wgpu::Face>,
}

impl RenderPipelineBuilder {
    pub fn new() -> Self {
        Self {
            label: "default-pipeline".to_string(),
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

    /// Set the primitive topology enum variant
    pub fn with_topology(mut self, top: wgpu::PrimitiveTopology) -> Self {
        self.topology = top;
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
}

impl ResourceBuilder for RenderPipelineBuilder {
    type Output = wgpu::RenderPipeline;
    type Context = RenderPipelineContext;

    /// Construct the render pipeline with the settings provided through the stored template
    fn build(&self, context: Arc<RenderPipelineContext>) -> Result<Self::Output, String> {
        let layout_refs: Vec<&wgpu::BindGroupLayout> = context.bg_layouts
            .iter()
            .map(|arc| arc.as_ref()) // or just &**arc
            .collect();

        let pipeline_layout = context.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some(&self.label),
                bind_group_layouts: &layout_refs,
                immediate_size: 0,
            }
        );
        
        let vertex = wgpu::VertexState {
            module: &context.shader,
            entry_point: Some(&context.shader_spec.vs_main),
            buffers: &context.shader_spec.build_vertex_layouts(),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        };

        let fragment = Some(wgpu::FragmentState {
            module: &context.shader,
            entry_point: Some(&context.shader_spec.fs_main),
            targets: &[Some(wgpu::ColorTargetState {
                format: context.format,
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

        let render_pipeline = context.device.create_render_pipeline(
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

        println!("[Render Pipeline] Created new render pipeline with label '{}'", self.label);

        Ok(render_pipeline)
    }
}
