use std::{ borrow::Cow, sync::Arc };
use wgpu::Device;

use super::vertex::Vertex;
use super::traits::{ Handler, ResourceDescriptor };
use super::registry::ResourceRegistry;


/// lightwight configuration struct for WGPU pipelines
#[derive(Clone)]
pub struct RenderPipelineConfig {
    /// unique identifier for the pipeline
    pub name: String,
    /// the path to the pipeline's shader source file
    pub path: String,
    /// the name of the entry function into the vertex stage 
    pub vert_main: String,
    /// the name of the entry function into the fragment stage
    pub frag_main: String,
}

impl ResourceDescriptor for RenderPipelineConfig {
    type Key = String;

    fn get_key(&self) -> &Self::Key {
        &self.name
    }
}

/// Container for rendering pipelines.
/// 
/// Allows for asyncronous creation of pipelines and shaders
/// 
/// Uses a mcsp channel under the hood for message passing between the main and worker threads.
/// 
/// Stores references to pipelines that can be requested by name during runtime for hot reloading
pub struct RenderPipelineHandler {
    device: Arc<Device>,
    pipeline_registry: ResourceRegistry<String, wgpu::RenderPipeline>,
    // config_registry: ResourceRegistry<String, RenderPipelineConfig>,
    surface_format: wgpu::TextureFormat,
}

impl RenderPipelineHandler {
    /// Create a new Pipeline Handler
    /// 
    /// device: the GPU device for which to create pipelines with
    /// 
    /// format: the texture format for the pipelines
    pub fn new(device: &Arc<Device>, format: wgpu::TextureFormat) -> Self {
        Self { 
            device: Arc::clone(&device),
            pipeline_registry: ResourceRegistry::new(),
            // config_registry: ResourceRegistry::new(),
            surface_format: format,
        }
    }
}

impl Handler<RenderPipelineConfig, wgpu::RenderPipeline> for RenderPipelineHandler {
    fn request_new(&mut self, pipeline_config: &RenderPipelineConfig) {
        let config_cpy = pipeline_config.clone();
        let device_cpy = Arc::clone(&self.device);
        let surf_format = self.surface_format;

        self.pipeline_registry.request_new(
            &config_cpy.name.clone(), 
            create_pipeline(device_cpy, config_cpy, surf_format)
        );
    }

    fn sync(&mut self) {
        self.pipeline_registry.sync();
    }

    fn get(&self, pipeline_name: &String) -> Option<&wgpu::RenderPipeline> {
        self.pipeline_registry.get(pipeline_name)
    }

    fn remove(&mut self, pipeline_name: &String) {
        self.pipeline_registry.remove(pipeline_name);
    }
}

/// creates a new rendering pipeline using a gpu device, pipeline config, and texture format
async fn create_pipeline(
    device: Arc<wgpu::Device>, 
    config: RenderPipelineConfig, 
    format: wgpu::TextureFormat,
) -> Result<wgpu::RenderPipeline, String> {
    let shader_source = match std::fs::read_to_string(&config.path) {
        Ok(source) => source,
        Err(e) => {
            return Err(format!("Failed to read shader file '{}': {e}", &config.path));
        }
    };

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(format!("[ShaderModule]{}", config.name).as_str()),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
    });

    let pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(format!("[Pipeline]{}", config.name).as_str()),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some(&config.vert_main),
            buffers: &[Vertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(&config.frag_main),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    });

    return Ok(render_pipeline);
}