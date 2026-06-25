#![allow(dead_code)]
use wgpu::TextureView;
use winit::window::Window;
use std::sync::Arc;

use crate::{
    bind_group::*, 
    buffer::{BufferBuilder, BufferContext}, 
    core::WgpuCore, 
    font::{FontAsset, FontBuilder}, 
    geometry::GeometrySignature, 
    handler::ResourceHandler, 
    init_state::{InitMode, StateInit}, 
    presets::{ShaderSpecPreset, TextureSampler}, 
    primitive::RenderInfo, 
    render_pipeline::{RenderPipelineBuilder, RenderPipelineContext}, 
    renderer::{DrawCommand, RenderContext}, 
    shader::ShaderSpec, 
    texture::{SamplerBuilder, TextureBuilder, TextureContext}, 
    vertex::VertexLayoutBuilder
};

/// Group binding number for global uniforms
pub const GLOBAL_UNIFORMS: u32 = 0;
/// Group binding number for material uniforms
pub const MATERIAL_UNIFORMS: u32 = 1;

/// vertex slot number for vertex buffers
pub const VERTEX_BUFFER: u32 = 0;
/// vertex slot number for instance buffers
pub const INSTANCE_BUFFER: u32 = 1;

/// Specfies the type of resource stored
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum ResourceType {
    /// A uniform buffer
    Uniform,
    /// A vertex buffer with vertex step mode
    Vertex(VertexLayoutBuilder),
    /// A vertex buffer with instance step mode
    Instance(VertexLayoutBuilder),
    /// An index buffer
    Index,
    /// A storage buffer
    Storage,
    /// A texture view
    Texture,
    /// A texture sampler
    Sampler,
    /// a bind group
    BindGroup,
    /// a font asset (atlas + metrics)
    Font,
}

/// The hold policy for gpu resources. 
/// These are used to specify how long a resource should stay in memory before being deallocated due to not being used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldPolicy {
    /// Resource is kept for a short amount of time (e.g. 5 seconds).
    Volatile,
    /// Resource is kept for a medium short amount of time (e.g. 10 seconds)
    Dynamic,
    /// Resource is kept for medium long amount of time (e.g. 15 seconds)
    Transient,
    /// Resource is kept for a long amount of time (e.g. 30 seconds)
    Persistent,
    /// Resource lives the entire runtime duration
    Indefinite,
}

impl HoldPolicy {
    /// get this hold policy in seconds
    pub fn as_seconds(self) -> Option<u64> {
        match self {
            HoldPolicy::Indefinite => None,
            HoldPolicy::Volatile => Some(3),
            HoldPolicy::Dynamic => Some(5),
            HoldPolicy::Transient => Some(15),
            HoldPolicy::Persistent => Some(30),
        }
    }
}

/// Specifies the scope for which a resource should be namespaced (allows different levels of resource sharing)
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum ResourceScope {
    /// Resources are scoped to individual primitives and not shared (best used for buffers)
    Primitive,
    /// Resources are scoped to materials and shared between primitives (best used for textures)
    Material,
    /// Resources are globally scoped and are shared everywhere (best used for samplers/fonts)
    Global
}

/// The identifer of a gpu resource, including its unique key and scope
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ResourceID {
    /// The unqiue key for a resource
    pub key: String,
    /// The namespaced scope of a resource
    pub scope: ResourceScope,
    /// The resource type
    pub r_type: ResourceType
}

/// The specific binding of a resource when used in a bind group
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ResourceBinding {
    /// the id of the resource
    pub id: ResourceID,
    /// the bind slot the resource will be placed in a matching shader
    pub slot: u32,
}

/// Encapsulates the ids for geometry buffers
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct GeometryID {
    pub vertex_id: ResourceID,
    pub index_id: ResourceID,
    pub indices: u32,
}

/// Update resource data using suballocation
#[derive(Clone, Debug)]
pub struct ResourceUpdate {
    pub data: Vec<u8>,
    pub offset: u64,
}

/// Represents the entire WebGPU rendering context. 
/// 
/// Coordinates syncronization of gpu resources created on handler worker threads with the main thread.
/// 
/// Exposes methods to generate and update resources, and render objects.
pub struct WgpuContext {
    core: WgpuCore,

    // resource layouts/metadata
    bg_layout_handler: ResourceHandler<BindGroupLayoutBuilder, Arc<wgpu::BindGroupLayout>>,
    vertex_layout_handler: ResourceHandler<VertexLayoutBuilder, Arc<wgpu::VertexBufferLayout<'static>>>,
    pipeline_handler: ResourceHandler<RenderPipelineBuilder, wgpu::RenderPipeline>,
    shader_spec_handler: ResourceHandler<String, ShaderSpec>,
    shader_mod_handler: ResourceHandler<String, Arc<wgpu::ShaderModule>>,

    /// resources
    buffer_handler: ResourceHandler<ResourceID, Arc<wgpu::Buffer>>,
    font_handler: ResourceHandler<ResourceID, Arc<FontAsset>>,
    texture_handler: ResourceHandler<ResourceID, Arc<TextureView>>,
    sampler_handler: ResourceHandler<ResourceID, Arc<wgpu::Sampler>>,
    bindgroup_handler: ResourceHandler<ResourceID, BindGroupAsset>,
}

impl WgpuContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let core = WgpuCore::new(window).await;

        let sampler_handler = WgpuContext::init_samplers(&core);
        let shader_spec_handler = WgpuContext::init_shader_specs();
        
        Self {
            core,
            sampler_handler,
            shader_spec_handler,
            bg_layout_handler: ResourceHandler::new(),
            vertex_layout_handler: ResourceHandler::new(),
            pipeline_handler: ResourceHandler::new(),
            shader_mod_handler: ResourceHandler::new(),
            buffer_handler: ResourceHandler::new(),
            texture_handler: ResourceHandler::new(),
            font_handler: ResourceHandler::new(),
            bindgroup_handler: ResourceHandler::new(),
        }
    }

    /// pre-initialize known shader specifications 
    fn init_shader_specs() -> ResourceHandler<String, ShaderSpec> {
        let colored_sprite = ShaderSpecPreset::ColoredSprite;
        let textured_sprite = ShaderSpecPreset::TexturedSprite;

        let mut spec_handler = ResourceHandler::new();
        spec_handler.request_new(&colored_sprite.path(), &colored_sprite.get(), Arc::new(()));
        spec_handler.request_new(&textured_sprite.path(), &textured_sprite.get(), Arc::new(()));

        spec_handler
    }

    /// pre-intitialize common samplers
    fn init_samplers(core: &WgpuCore) -> ResourceHandler<ResourceID, Arc<wgpu::Sampler>> {
        let mut sampler_handler = ResourceHandler::new();
        let _ = sampler_handler.request_wait(
            &ResourceID { 
                key: TextureSampler::NearestClampToEdge.label(),
                scope: ResourceScope::Global,
                r_type: ResourceType::Sampler,
            }, 
            &TextureSampler::NearestClampToEdge.get(), 
            Arc::clone(&core.device)
        );
        let _ = sampler_handler.request_wait(
            &ResourceID { 
                key: TextureSampler::NearestRepeat.label(),
                scope: ResourceScope::Global,
                r_type: ResourceType::Sampler
            },
            &TextureSampler::NearestRepeat.get(), 
            Arc::clone(&core.device)
        );

        sampler_handler
    }

    /// initialize resources prior to rendering state
    pub fn init_resources(&mut self, init_state: StateInit) {
        for rpip_cmd in init_state.get_rpip_cmds() {
            let render_info = RenderInfo {
                shader_path: rpip_cmd.shader_path.clone(),
                pipeline: rpip_cmd.builder.clone()
            };
            self.process_pipeline(&render_info, InitMode::Deferred);
        }
        
        for bgl_cmd in init_state.get_bgl_cmds() {
            match bgl_cmd.mode {
                InitMode::Immediate => {
                    let _ = self.bg_layout_handler.request_wait(&bgl_cmd.builder, &bgl_cmd.builder, Arc::clone(&self.core.device));
                },
                InitMode::Deferred => {
                    self.bg_layout_handler.request_new(&bgl_cmd.builder, &bgl_cmd.builder, Arc::clone(&self.core.device));
                }
            }
        }
    }

    /// initialize a shader specifcation
    pub fn process_shader_spec(&mut self, path: &String) {
        if let Some(spec) = self.shader_spec_handler.get(path) {
            for layout in &spec.bg_layouts {
                if self.bg_layout_handler.is_ready(layout) { continue; }

                let device_cpy = self.core.device.clone();
                self.bg_layout_handler.request_new(&layout, layout, device_cpy);
            }

            for vt_layout in &spec.vt_layouts {
                self.vertex_layout_handler.request_new(&vt_layout, vt_layout, Arc::new(()));
            }
        } else {
            let builder = ShaderSpecPreset::from_known_path(path).unwrap();
            self.shader_spec_handler.request_new(path, &builder, Arc::new(()));
        }
    }

    /// Initialize a new pipeline request
    pub fn process_pipeline(&mut self, render_info: &RenderInfo, mode: InitMode) {
        if self.pipeline_handler.contains(&render_info.pipeline) { return; }

        let shader_spec = match self.shader_spec_handler.get(&render_info.shader_path) {
            Some(spec) => spec,
            None => { return; }
        };

        let shader_mod = match self.shader_mod_handler.get(&render_info.shader_path) {
            Some(module ) => module,
            None => { 
                let device_cpy = self.core.device.clone();
                self.shader_mod_handler.request_new(&render_info.shader_path, &shader_spec.shader_builder, device_cpy);
                return; 
            }
        };

        let mut bg_layouts = Vec::new();
        for id in &shader_spec.bg_layouts {
            if let Some(layout) = self.bg_layout_handler.get(&id) {
                bg_layouts.push(Arc::clone(layout));
            }
        }

        let mut vertex_layouts = Vec::new();
        let mut vt_layouts_ready = true;
        for vt_builder in &shader_spec.vt_layouts {
            if let Some(layout) = self.vertex_layout_handler.get(vt_builder) {
                vertex_layouts.push(Arc::clone(layout));
            } else {
                vt_layouts_ready = false;
            }
        }

        let bgl_ready = bg_layouts.len() == shader_spec.bg_layouts.len();

        if bgl_ready && vt_layouts_ready {
            let rpip_context = Arc::new(
                RenderPipelineContext {
                    device: Arc::clone(&self.core.device),
                    bg_layouts,
                    format: self.core.config.format.clone(),
                    shader: shader_mod.clone(),
                    shader_spec: shader_spec.clone(),
                    vertex_layouts
                }
            );

            match mode {
                InitMode::Immediate => {
                    let _ = self.pipeline_handler.request_wait(
                        &render_info.pipeline, 
                        &render_info.pipeline, 
                        Arc::clone(&rpip_context)
                    );
                },
                InitMode::Deferred => {
                    self.pipeline_handler.request_new(
                        &render_info.pipeline, 
                        &render_info.pipeline, 
                        Arc::clone(&rpip_context)
                    );
                }
            }
        }
    }

    /// initialize a new bind group request 
    pub fn process_bind_group(&mut self, group_id: &ResourceID, layout_id: &BindGroupLayoutBuilder, bindings: Vec<ResourceBinding>) {
        if self.bindgroup_handler.contains(group_id) { return; }
        
        let layout = match self.bg_layout_handler.get(layout_id) {
            Some(layout) => layout,
            None => { return; }
        };

        // println!("Creating new bind group with layout: {}", layout_id.label);

        let mut dependencies: Vec<ResourceID> = Vec::new();
        let mut resource_pairs = Vec::with_capacity(bindings.len());
        for binding in &bindings {
            // check for buffer
            if let Some(buffer) = self.buffer_handler.get(&binding.id) {
                resource_pairs.push((binding.slot, BindGroupResource::Buffer(Arc::clone(buffer))))
            }
            // check for texture
            if let Some(texture_view) = self.texture_handler.get(&binding.id) {
                resource_pairs.push((binding.slot, BindGroupResource::Texture(Arc::clone(&texture_view))));
            }
            // check for font atlas
            if let Some(font_asset) = self.font_handler.get(&binding.id) {
                resource_pairs.push((binding.slot, BindGroupResource::Texture(Arc::clone(&font_asset.atlas))));
            }
            // check for sampler
            if let Some(sampler) = self.sampler_handler.get(&binding.id) {
                resource_pairs.push((binding.slot, BindGroupResource::Sampler(Arc::clone(&sampler))));
            }
            dependencies.push(binding.id.clone());
        }

        // println!("found resources: {:#?}, expected respources: {:#?}", resource_pairs, bindings);

        // all resources found, safe to create bind group
        if resource_pairs.len() == bindings.len() {
            let builder = BindGroupBuilder::new()
                .with_label(&group_id.key)
                .with_resources(resource_pairs);

            let context = Arc::new(BindGroupContext {
                device: Arc::clone(&self.core.device),
                layout: Arc::clone(&layout),
                dependencies
            });

            self.bindgroup_handler.request_new(&group_id, &builder, context);
        }
    }

    /// initialize a new buffer request
    pub fn process_buffer(&mut self, id: &ResourceID, builder: &BufferBuilder) {
        if self.contains_buffer(id) { return; }

        let context = Arc::new(BufferContext {
            device: Arc::clone(&self.core.device),
            queue: Arc::clone(&self.core.queue)
        });

        self.buffer_handler.request_new(&id, builder, context);
    }

    /// Check if the context has a buffer with the given id stored.
    pub fn contains_buffer(&self, id: &ResourceID) -> bool{
        return self.buffer_handler.contains(id)
    }

    pub fn contains_resource(&self, id: &ResourceID) -> bool {
        match id.r_type {
            ResourceType::Texture => self.texture_handler.contains(id),
            ResourceType::Sampler => self.sampler_handler.contains(id),
            ResourceType::Font => self.font_handler.contains(id),
            ResourceType::BindGroup => self.bindgroup_handler.contains(id),
    
            ResourceType::Uniform | ResourceType::Index | ResourceType::Storage |
            ResourceType::Vertex(_) | ResourceType::Instance(_) => {
                self.buffer_handler.contains(id)
            },
        }
    }

    /// initialize a new texture request
    pub fn process_texture(&mut self, id: &ResourceID, builder: &TextureBuilder) {
        if self.texture_handler.contains(id) { return; }

        let context = Arc::new(TextureContext {
            device: Arc::clone(&self.core.device),
            queue: Arc::clone(&self.core.queue),
        });

        self.texture_handler.request_new(&id, builder, context);
    }

    /// Get the font asset associated with the provided resource id, if exists.
    pub fn get_font(&self, id: &ResourceID) -> Option<Arc<FontAsset>> {
        self.font_handler.get(id).cloned()
    }

    /// Initialize a new font request
    pub fn process_font(&mut self, id: &ResourceID, builder: &FontBuilder) {
        if self.font_handler.contains(id) { return; }

        let font_context = Arc::new(TextureContext {
            queue: self.core.queue.clone(),
            device: self.core.device.clone()
        });

        self.font_handler.request_new(&id, builder, font_context);
    }

    /// initialize a new sampler request
    pub fn process_sampler(&mut self, id: &ResourceID, builder: &SamplerBuilder) {
        if self.sampler_handler.contains(id) { return; }
        
        self.sampler_handler.request_new(&id, builder, Arc::clone(&self.core.device));
    }

    pub fn process_geometry(&mut self, geometry_sig: GeometrySignature) {
        let vtx_buffer_exists = self.buffer_handler.contains(&geometry_sig.ids.vertex_id);
        let idx_buffer_exists = self.buffer_handler.contains(&geometry_sig.ids.index_id);
        if vtx_buffer_exists && idx_buffer_exists { return; }
        
        let buffer_context = Arc::new(BufferContext {
            device: Arc::clone(&self.core.device),
            queue: Arc::clone(&self.core.queue)
        });

        self.buffer_handler.request_new(&geometry_sig.ids.vertex_id, &geometry_sig.vertex_builder, Arc::clone(&buffer_context));
        self.buffer_handler.request_new(&geometry_sig.ids.index_id, &geometry_sig.index_builder, Arc::clone(&buffer_context));
    }

    /// Prepare the context for the next frame
    pub fn prepare_next_frame(&mut self) {
        self.bg_layout_handler.sync();
        self.pipeline_handler.sync();
        self.bindgroup_handler.sync();
        self.buffer_handler.sync();
        self.texture_handler.sync();
        self.font_handler.sync();
        self.sampler_handler.sync();
        self.shader_spec_handler.sync();
        self.shader_mod_handler.sync();
        self.vertex_layout_handler.sync();

        self.core.window.request_redraw();
    }

    /// resize the surface that the context renders to (also resizes the window)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.core.resize(width, height);
    }

    /// process update commands
    pub fn update_resource(&mut self, id: &ResourceID, update: ResourceUpdate) {
        if let Some(buffer) = self.buffer_handler.get(id) {
            if update.offset + (update.data.len() as u64) > buffer.size() {
                println!("[WgpuContext Error] Cannot write data to buffer because the size of the update is too large.");
                return; 
            }

            self.core.queue.write_buffer(buffer, update.offset, &update.data);
        }
        // add texture check when ready
    }

    /// render commands given to the renderer instance
    pub fn render(&mut self, render_ctx: RenderContext) -> anyhow::Result<()> {
        if !self.core.is_surface_configured() {
            return Ok(());
        }

        // verify camera existence
        let global_bg = match self.bindgroup_handler.get(&render_ctx.global_id) {
            Some(bg_asset) => {
                self.prepare_bind_group(bg_asset);
                &bg_asset.bg
            },
            None => return Ok(()) // if the camera bind group is not ready, we can't draw anything
        };

        // prepare output and render pass
        let output = self.core.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.core.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") }
        );

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(render_ctx.bg_color),
                            store: wgpu::StoreOp::Store,
                        }
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                }
            );

            // draw meshes to current texture
            render_pass.set_bind_group(GLOBAL_UNIFORMS, global_bg, &[]);
            self.draw_instances(&render_ctx.draw_cmds, &mut render_pass);
        }

        self.core.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn draw_instances(&self, draw_cmds: &Vec<DrawCommand>, render_pass: &mut wgpu::RenderPass) {
        for command in draw_cmds {
            let vert_status = self.buffer_handler.get(&command.geometry_id.vertex_id);
            let idx_status = self.buffer_handler.get(&command.geometry_id.index_id);
            let pip_status = self.pipeline_handler.get(&command.pipeline_id);
            let u_mat_status = self.bindgroup_handler.get(&command.material_id);
            let u_instance_status = self.buffer_handler.get(&command.instance_id);

            // println!("geometry ready? {}", vert_status.is_some() && idx_status.is_some());
            // println!("material ready? {}", u_mat_status.is_some());
            // println!("instances ready? {}", u_instance_status.is_some());
            // println!("pipeline ready? {}", pip_status.is_some());

            if let (Some(vtx_buffer),
                    Some(idx_buffer),
                    Some(pipeline), 
                    Some(mat_bg_asset),
                    Some(u_instance)) = (vert_status, idx_status, pip_status, u_mat_status, u_instance_status) 
            {
                self.prepare_bind_group(mat_bg_asset);

                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(MATERIAL_UNIFORMS, &mat_bg_asset.bg, &[]);
                render_pass.set_vertex_buffer(VERTEX_BUFFER, vtx_buffer.slice(..));
                render_pass.set_vertex_buffer(INSTANCE_BUFFER, u_instance.slice(..));
                render_pass.set_index_buffer(idx_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..command.geometry_id.indices, 0, 0..command.instances);
            }
        }
    }

    fn prepare_bind_group(&self, bind_group: &BindGroupAsset) {
        for dep in bind_group.dependencies.iter() {
            self.texture_handler.mark_accessed(dep);
            self.buffer_handler.mark_accessed(dep);
            self.sampler_handler.mark_accessed(dep);
        }
    }
}