use std::{borrow::Cow, collections::HashMap, sync::{Arc, mpsc}};
use wgpu::Device;
use tokio::task;

use super::vertex::Vertex;


/// lightwight configuration struct for shader modules
#[derive(Clone)]
pub struct ShaderConfig {
    /// unique identifier for the shader
    pub name: String,
    /// the path to the shader source file
    pub path: String,
    /// the name of the entry function into the vertex stage 
    pub vert_main: String,
    /// the name of the entry function into the fragment stage
    pub frag_main: String,
}

/// Container for rendering pipelines.
/// 
/// Allows for asyncronous creation of pipelines and shaders
/// 
/// Uses a mcsp channel under the hood for message passing between the main and worker threads.
/// 
/// Stores references to pipelines that can be requested by name during runtime for hot reloading
pub struct PipelineHandler {
    device: Arc<Device>,
    render_pipelines: HashMap<String, Option<wgpu::RenderPipeline>>,
    shader_configs: HashMap<String, ShaderConfig>,
    surface_format: wgpu::TextureFormat,
    tx: mpsc::Sender<(String, wgpu::RenderPipeline)>,
    rx: mpsc::Receiver<(String, wgpu::RenderPipeline)>
}

impl PipelineHandler {
    pub fn new(device: &Arc<Device>, format: wgpu::TextureFormat) -> Self {
        let (tx, rx) = mpsc::channel();
        Self { 
            device: Arc::clone(&device),
            render_pipelines: HashMap::new(),
            shader_configs: HashMap::new(),
            surface_format: format,
            tx, rx
        }
    }

    /// requests a rendering pipeline to be created, if not already done.
    pub fn request_pipeline(&mut self, shader_config: ShaderConfig) {
        if self.render_pipelines.contains_key(&shader_config.name) {
            return; // pipeline already requested
        }

        let config = shader_config.clone();
        let device = Arc::clone(&self.device);
        let format = self.surface_format;
        let tx = self.tx.clone();

        self.render_pipelines.insert(shader_config.name.clone(), None);
        self.shader_configs.insert(shader_config.name.clone(), shader_config);

        // have the pipeline be created in a separate thread
        task::spawn(async move {
            let shader_source = std::fs::read_to_string(&config.path)
                .expect("Failed to read shader file");

            let pipeline = create_pipeline(&device, &config, format, shader_source).await;
            let _ = tx.send((config.name, pipeline));
        });
    }

    /// checks if any pipelines have completed compilation, and if so, updates the pipeline map. Should be called regularly (e.g. once per frame)
    pub fn check_ready_pipelines(&mut self) {
        while let Ok((name, pipeline)) = self.rx.try_recv() {
            self.render_pipelines.insert(name, Some(pipeline));
        }
    }

    /// given a pipeline name, returns an Option with a reference to the corresponding RenderPipeline instance. the None variant signifies that the pipeline was not created or has not finished compliling
    pub fn get_pipeline(&self, pipeline_name: &str) -> Option<&wgpu::RenderPipeline> {
        return self.render_pipelines.get(pipeline_name)?.as_ref();
    }
}

/// creates a new rendering pipeline using a gpu device, shader config, and texture format
async fn create_pipeline(
    device: &wgpu::Device, 
    shader_config: &ShaderConfig, 
    format: wgpu::TextureFormat,
    shader_source: String,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(format!("[ShaderModule]{}", shader_config.name).as_str()),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
    });

    let pipeline_layout = device.create_pipeline_layout(
        &wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(format!("[Pipeline]{}", shader_config.name).as_str()),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some(&shader_config.vert_main),
            buffers: &[Vertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(&shader_config.frag_main),
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

    return render_pipeline;
}