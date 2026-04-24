#![allow(dead_code)]
use wgpu::TextureView;
use winit::window::Window;
use std::sync::Arc;

use crate::graphics::{
    bind_group::{BindGroupBuilder, BindGroupContext, BindGroupLayoutBuilder, BindGroupResource}, buffer::BufferContext, core::WgpuCore, handler::{ResourceHandler, ResourceStatus}, init_state::{InitMode, StateInit}, mesh::MeshBuffer, presets::TextureSampler, render_pipeline::{RenderPipelineBuilder, RenderPipelineContext}, renderer::{CreateCommand, Renderer, UpdateCommand}, texture::{TextureBuilder, TextureContext}, tracker::ResourceTracker
};

/// Group binding number for global uniforms
pub const GLOBAL_UNIFORMS: u32 = 0;
/// Group binding number for material uniforms
pub const MATERIAL_UNIFORMS: u32 = 1;
/// Group binding number for instance uniforms
pub const INSTANCE_UNIFORMS: u32 = 2;

/// Specifies the scope for which a resource should be namespaced (allows different levels of resource sharing)
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum ResourceScope {
    /// Resources are scoped to individual entities; no sharing (best used for buffers)
    Entity,
    /// Resources are scoped to materials; shared between entities (best used for textures)
    Material,
    /// Resources are globally scoped; shared everywhere (best used for samplers)
    Global
}

/// The idenitifer of a gpu resource, including its unique key and scope
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ResourceID {
    /// The unique key for a resource
    pub key: String,
    /// The namespaced scope of a resource
    pub scope: ResourceScope,
}

/// The specific binding of a resource when used in a bind group
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ResourceBinding {
    pub id: ResourceID,
    pub slot: u32,
}

/// Represents the entire WebGPU rendering context. 
/// 
/// Coordinates syncronization of gpu resources created on handler worker threads with the main thread.
/// 
/// Accepts commands generated to generate and update resources, and render objects.
pub struct WgpuContext {
    core: WgpuCore,
    tracker: ResourceTracker,

    layout_handler: ResourceHandler<BindGroupLayoutBuilder, Arc<wgpu::BindGroupLayout>>,
    pipeline_handler: ResourceHandler<RenderPipelineBuilder, wgpu::RenderPipeline>,

    buffer_handler: ResourceHandler<ResourceID, Arc<wgpu::Buffer>>,
    texture_handler: ResourceHandler<ResourceID, Arc<TextureView>>,
    sampler_handler: ResourceHandler<ResourceID, Arc<wgpu::Sampler>>,

    mesh_handler: ResourceHandler<u32, MeshBuffer>,
    bindgroup_handler: ResourceHandler<String, wgpu::BindGroup>,
}

impl WgpuContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let core = WgpuCore::new(window).await;

        let sampler_handler = WgpuContext::init_samplers(&core);

        Self {
            core,
            sampler_handler,
            layout_handler: ResourceHandler::new(),
            mesh_handler: ResourceHandler::new(),
            bindgroup_handler: ResourceHandler::new(),
            pipeline_handler: ResourceHandler::new(),
            buffer_handler: ResourceHandler::new(),
            texture_handler: ResourceHandler::new(),
            tracker: ResourceTracker::new()
        }
    }

    fn init_samplers(core: &WgpuCore) -> ResourceHandler<ResourceID, Arc<wgpu::Sampler>> {
        let mut sampler_handler = ResourceHandler::new();
        let _ = sampler_handler.request_wait(
            &ResourceID { 
                key: TextureSampler::NearestClampToEdge.as_key(), 
                scope: ResourceScope::Global 
            }, 
            &TextureSampler::NearestClampToEdge.get(), 
            Arc::clone(&core.device)
        );
        let _ = sampler_handler.request_wait(
            &ResourceID { 
                key: TextureSampler::NearestRepeat.as_key(), 
                scope: ResourceScope::Global 
            }, 
            &TextureSampler::NearestRepeat.get(), 
            Arc::clone(&core.device)
        );

        sampler_handler
    }

    /// initialize resources prior to rendering state
    pub fn init_resources(&mut self, init_state: StateInit) {
        for rpip_cmd in init_state.get_rpip_cmds() {
            self.init_pipeline(&rpip_cmd.builder, InitMode::Deferred);
        }
        
        for bgl_cmd in init_state.get_bgl_cmds() {
            match bgl_cmd.mode {
                InitMode::Immediate => {
                    let _ = self.layout_handler.request_wait(&bgl_cmd.builder, &bgl_cmd.builder, Arc::clone(&self.core.device));
                },
                InitMode::Deferred => {
                    self.layout_handler.request_new(&bgl_cmd.builder, &bgl_cmd.builder, Arc::clone(&self.core.device));
                }
            }
        }
    }

    /// Initialize a new pipeline request
    fn init_pipeline(&mut self, builder: &RenderPipelineBuilder, mode: InitMode) {
        let mut layouts = Vec::new();
        for id in &builder.get_layout_ids() {
            if let Some(layout) = self.layout_handler.get(&id) {
                layouts.push(Arc::clone(layout));
            }
        }

        if layouts.len() == builder.get_layout_ids().len() {
            let rpip_context = Arc::new(
                RenderPipelineContext {
                    device: Arc::clone(&self.core.device),
                    layouts: layouts,
                    format: self.core.config.format.clone()
                }
            );

            self.tracker.pipelines.insert(builder.clone());
            match mode {
                InitMode::Immediate => {
                    let _ = self.pipeline_handler.request_wait(
                        &builder, 
                        builder, 
                        Arc::clone(&rpip_context)
                    );
                },
                InitMode::Deferred => {
                    self.pipeline_handler.request_new(
                        &builder, 
                        builder, 
                        Arc::clone(&rpip_context)
                    );
                }
            }
        }
    }

    /// initialize a new bind group request 
    fn init_bind_group(&mut self, group_id: &String, layout_id: &BindGroupLayoutBuilder, bindings: Vec<ResourceBinding>) {
        if !self.layout_handler.is_ready(layout_id) {
            return;
        }

        let mut resource_pairs = Vec::with_capacity(bindings.len());
        for binding in &bindings {
            // check for buffer
            if let Some(buffer) = self.buffer_handler.get(&binding.id) {
                resource_pairs.push((binding.slot.clone(), BindGroupResource::Buffer(Arc::clone(buffer))))
            }
            // check for texture
            if let Some(texture_view) = self.texture_handler.get(&binding.id) {
                resource_pairs.push((binding.slot.clone(), BindGroupResource::Texture(Arc::clone(&texture_view))));
            }
            // check for sampler
            if let Some(sampler) = self.sampler_handler.get(&binding.id) {
                resource_pairs.push((binding.slot.clone(), BindGroupResource::Sampler(Arc::clone(&sampler))));
            }
        }

        // println!("found resources: {:?}, expected respources: {:?}", resource_pairs, bindings);

        // all resources found, safe to create bind group
        if resource_pairs.len() == bindings.len() {
            let layout = self.layout_handler.get(layout_id).unwrap();
            let builder = BindGroupBuilder::new()
                .with_label(&group_id)
                .with_resources(resource_pairs);

            let context = Arc::new(BindGroupContext {
                device: Arc::clone(&self.core.device),
                layout: Arc::clone(&layout)
            });

            self.tracker.bind_groups.insert(group_id.clone());
            self.bindgroup_handler.request_new(&group_id, &builder, context);
        }
    }

    /// initialize a new texture request
    fn init_texture(&mut self, key: &ResourceID, builder: &TextureBuilder) {
        let context = Arc::new(TextureContext {
            device: Arc::clone(&self.core.device),
            queue: Arc::clone(&self.core.queue),
        });

        self.tracker.buffers.insert(key.clone());
        self.texture_handler.request_new(&key, builder, context);
    }

    /// Prepare the context for the next frame
    pub fn prepare_next_frame(&mut self) {
        self.layout_handler.sync();
        self.mesh_handler.sync();
        self.pipeline_handler.sync();
        self.bindgroup_handler.sync();
        self.buffer_handler.sync();
        self.texture_handler.sync();
        self.sampler_handler.sync();

        self.core.window.request_redraw();
    }

    /// Swap the stored resource tracker with another one. This enables double buffering on resource tracking.
    pub fn swap_trackers(&mut self, mut other: ResourceTracker) -> ResourceTracker {
        other.clear();
        other.copy_from(&self.tracker);

        std::mem::swap(&mut self.tracker, &mut other);

        other
    }

    /// resize the surface that the context renders to (also resizes the window)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.core.resize(width, height);
    }

    /// Create resources from a list of create commands
    pub fn create_resources(&mut self, create_cmds: &Vec<CreateCommand>) {
        for create_cmd in create_cmds {
            match create_cmd {
                CreateCommand::Mesh { id, builder } => {
                    let context = Arc::new(BufferContext {
                        device: Arc::clone(&self.core.device),
                        queue: Arc::clone(&self.core.queue)
                    });

                    self.tracker.meshes.insert(id.clone());
                    self.mesh_handler.request_new(&id, builder, context);
                },
                CreateCommand::Buffer { id, builder } => {
                    let context = Arc::new(BufferContext {
                        device: Arc::clone(&self.core.device),
                        queue: Arc::clone(&self.core.queue)
                    });

                    self.tracker.buffers.insert(id.clone());
                    self.buffer_handler.request_new(&id, builder, context);
                },
                CreateCommand::Texture { id, builder } => {
                    self.init_texture(&id, builder);
                }
                CreateCommand::Sampler { id, builder } => {
                    self.tracker.samplers.insert(id.clone());
                    self.sampler_handler.request_new(&id, builder, Arc::clone(&self.core.device));
                }
                CreateCommand::BindGroupLayout { builder } => {
                    self.tracker.bg_layouts.insert(builder.clone());
                    self.layout_handler.request_new(&builder, builder, Arc::clone(&self.core.device));
                }
                CreateCommand::RenderPipeline { builder, mode } => {
                    self.init_pipeline(builder, mode.clone());
                },
                CreateCommand::BindGroup { id, layout_id, resource_ids } => {
                    self.init_bind_group(&id, &layout_id, resource_ids.clone());
                }
            }
        }
    }

    /// process update commands
    pub fn update_resources(&mut self, update_cmds: &Vec<UpdateCommand>) {
        for update_cmd in update_cmds {
            if let Some(buffer) = self.buffer_handler.get(&update_cmd.key) {
                self.core.queue.write_buffer(buffer, 0, &update_cmd.data);
            }
            // add texture check when ready
        }
    }

    /// render commands given to the renderer instance
    pub fn render(&mut self, renderer: Renderer) -> anyhow::Result<()> {
        if !self.core.is_surface_configured() {
            return Ok(());
        }

        // verify camera existence
        let camera_group = match self.bindgroup_handler.get(&renderer.get_camera_key()) {
            Some(data) => data,
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
                            load: wgpu::LoadOp::Clear(*renderer.get_bg_color()),
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
            render_pass.set_bind_group(GLOBAL_UNIFORMS, camera_group, &[]);
            self.draw_single(&renderer, &mut render_pass);
            self.draw_instances(&renderer, &mut render_pass);
        }

        self.core.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// draw meshes to the current texture using the provided render pass
    fn draw_single(&mut self, renderer: &Renderer, render_pass: &mut wgpu::RenderPass) {
        for draw_cmd in renderer.draw_cmds() {
            let mesh_status = self.mesh_handler.status_of(&draw_cmd.mesh_id);
            let pip_status = self.pipeline_handler.status_of(&draw_cmd.rpip_id.clone());
            let mat_u_status = self.bindgroup_handler.status_of(&draw_cmd.material_group.clone());
            let mesh_u_status = self.bindgroup_handler.status_of(&draw_cmd.entity_group.clone());

            // println!("mesh ready? {}", mesh_status.is_some());
            // println!("pipeline ready? {}", pip_status.is_some());
            // println!("material ready? {}", mat_u_status.is_some());
            // println!("transform ready? {}", mesh_u_status.is_some());

            if let (Some(ResourceStatus::Ready(mesh)), 
                    Some(ResourceStatus::Ready(pipeline)), 
                    Some(ResourceStatus::Ready(mat_uniforms)),
                    Some(ResourceStatus::Ready(mesh_uniforms))) = (mesh_status, pip_status, mat_u_status, mesh_u_status) 
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(MATERIAL_UNIFORMS, mat_uniforms, &[]);
                render_pass.set_bind_group(INSTANCE_UNIFORMS, mesh_uniforms, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }
    }

    fn draw_instances(&mut self, renderer: &Renderer, render_pass: &mut wgpu::RenderPass) {
        for insance_cmd in renderer.instance_cmds() {
            let mesh_status = self.mesh_handler.status_of(&insance_cmd.mesh_id);
            let pip_status = self.pipeline_handler.status_of(&insance_cmd.rpip_id.clone());
            let mat_u_status = self.bindgroup_handler.status_of(&insance_cmd.material_group.clone());
            let transforms_status = self.buffer_handler.status_of(&insance_cmd.transform_id);

            // println!("mesh ready? {}", mesh_status.is_some());
            // println!("pipeline ready? {}", pip_status.is_some());
            // println!("material ready? {}", mat_u_status.is_some());
            // println!("transforms ready? {}", transforms_status.is_some());

            if let (Some(ResourceStatus::Ready(mesh)), 
                    Some(ResourceStatus::Ready(pipeline)), 
                    Some(ResourceStatus::Ready(mat_uniforms)),
                    Some(ResourceStatus::Ready(instance_buffer))) = (mesh_status, pip_status, mat_u_status, transforms_status) 
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(MATERIAL_UNIFORMS, mat_uniforms, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..insance_cmd.instances);
            }
        }
    }
}