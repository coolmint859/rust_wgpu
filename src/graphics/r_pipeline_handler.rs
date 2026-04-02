use std::{ borrow::Cow, sync::Arc };
use wgpu::Device;

use super::vertex::Vertex;
use super::traits::{ Handler, ResourceDescriptor };

use super::presets::RenderPipelineConfig;
use super::registry::{ResourceRegistry, ResourceStatus};

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
    pub fn new(device: Arc<Device>, format: wgpu::TextureFormat) -> Self {
        Self { 
            device: Arc::clone(&device),
            pipeline_registry: ResourceRegistry::new(),
            // config_registry: ResourceRegistry::new(),
            surface_format: format,
        }
    }
}

impl Handler<RenderPipelineConfig, wgpu::RenderPipeline> for RenderPipelineHandler {
    fn request_new(&mut self, pipeline_config: Arc<RenderPipelineConfig>) {
        let config_cpy = Arc::clone(&pipeline_config);
        let device_cpy = Arc::clone(&self.device);
        let txtr_format = self.surface_format;

        self.pipeline_registry.request_new(
            pipeline_config.get_key(), 
            create_render_pipeline(device_cpy, config_cpy, txtr_format)
        );
    }

    fn request_wait(&mut self, pipeline_config: Arc<RenderPipelineConfig>) {
        let config_cpy = Arc::clone(&pipeline_config);
        let device_cpy = Arc::clone(&self.device);
        let txtr_format = self.surface_format;

        let result = self.pipeline_registry.request_wait(
            pipeline_config.get_key(),
            create_render_pipeline(device_cpy, config_cpy, txtr_format)
        );

        match result {
            Err(e) => eprintln!("Error creating render pipeline: {e}"),
            _ => {}
        }
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

    fn contains(&self, pipeline_name: &String) -> bool {
        return self.pipeline_registry.contains(pipeline_name);
    }

    fn status_of(&self, pipeline_name: &String) -> Option<&ResourceStatus<wgpu::RenderPipeline>> {
        return self.pipeline_registry.status_of(pipeline_name);
    }
}

/// creates a new rendering pipeline using a gpu device, pipeline config, and texture format
async fn create_render_pipeline(
    device: Arc<wgpu::Device>, 
    config: Arc<RenderPipelineConfig>, 
    format: wgpu::TextureFormat,
) -> Result<wgpu::RenderPipeline, String> {
    let shader_source = match std::fs::read_to_string(&config.shader_path) {
        Ok(source) => source,
        Err(e) => {
            return Err(format!("Failed to read shader file '{}': {e}", &config.shader_path));
        }
    };

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(format!("[Shader Module] {}@{}", config.name, config.shader_path).as_str()),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
    });

    let layout_refs: Vec<&wgpu::BindGroupLayout> = config.layouts
        .iter()
        .map(|arc| arc.as_ref()) // or just &**arc
        .collect();

    println!("bind group layouts: {:?}", config.layouts);

    let pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some(format!("[Pipeline Layout] {}", config.name).as_str()),
            bind_group_layouts: &layout_refs,
            immediate_size: 0,
        });

    let render_pipeline = device.create_render_pipeline(
        &wgpu::RenderPipelineDescriptor {
            label: Some(format!("[Render Pipeline] {}", config.name).as_str()),
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
        }
    );

    println!("Created pipeline '{}'", config.name);

    return Ok(render_pipeline);
}