#![allow(dead_code)]
use winit::window::Window;
use std::sync::Arc;

use crate::graphics::{
    bind_group::{BindGroupBuilder, BindGroupResource, ResourceID}, core::WgpuCore, gpu_resource::{GpuResourceHandler, ResourceStatus}, init_state::{InitMode, StateInit}, mesh::MeshBuffer, presets::BindingLayout, render_pipeline::RenderPipelineBuilder, renderer::{CreateCommand, Renderer, UpdateCommand}, tracker::ResourceTracker
};

/// group binding number for global uniforms
pub const GLOBAL_UNIFORMS: u32 = 0;
/// group binding number for material uniforms
pub const MATERIAL_UNIFORMS: u32 = 1;

/// Represents the entire WebGPU rendering context
pub struct WgpuContext {
    core: WgpuCore,
    layout_handler: GpuResourceHandler<String, Arc<wgpu::BindGroupLayout>>,
    mesh_handler: GpuResourceHandler<u32, MeshBuffer>,
    bindgroup_handler: GpuResourceHandler<String, wgpu::BindGroup>,
    pipeline_handler: GpuResourceHandler<String, wgpu::RenderPipeline>,
    buffer_handler: GpuResourceHandler<ResourceID, Arc<wgpu::Buffer>>,
    writer_tracker: ResourceTracker,
}

impl WgpuContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let core = WgpuCore::new(window).await;

        let mut layout_handler = GpuResourceHandler::new(Arc::clone(&core.device));

        let cs_builder = BindingLayout::ColoredSprite.get();
        let c2d_builder = BindingLayout::Camera2D.get();
        let _ = layout_handler.request_wait(&"colored-sprite".to_string(), &cs_builder);
        let _ = layout_handler.request_wait(&"camera-2d".to_string(), &c2d_builder);

        let mesh_handler = GpuResourceHandler::new(Arc::clone(&core.device));
        let bindgroup_handler = GpuResourceHandler::new(Arc::clone(&core.device));
        let pipeline_handler = GpuResourceHandler::new(Arc::clone(&core.device));
        let buffer_handler = GpuResourceHandler::new(Arc::clone(&core.device));
        
        Self {
            core,
            layout_handler,
            mesh_handler,
            bindgroup_handler,
            pipeline_handler,
            buffer_handler,
            writer_tracker: ResourceTracker::new()
        }
    }

    /// initialize resources prior to rendering state
    pub fn init_resources(&mut self, init_state: StateInit) {
        for mut rpip_cmd in init_state.get_rpip_cmds() {
            self.init_pipeline(&rpip_cmd.key, &mut rpip_cmd.builder, InitMode::Deferred);
        }
        
        for bgl_cmd in init_state.get_bgl_cmds() {
            match bgl_cmd.mode {
                InitMode::Immediate => {
                    let _ = self.layout_handler.request_wait(&bgl_cmd.key, &bgl_cmd.builder);
                },
                InitMode::Deferred => {
                    self.layout_handler.request_new(&bgl_cmd.key, &bgl_cmd.builder);
                }
            }
        }
    }

    fn init_pipeline(&mut self, key: &String, builder: &mut RenderPipelineBuilder, mode: InitMode) {
        let mut layouts = Vec::new();

        for id in &builder.get_layout_ids() {
            if let Some(layout) = self.layout_handler.get(&id) {
                layouts.push(Arc::clone(layout));
            } else {
                panic!("Required layout {} not found for pipeline {}", id, key);
            }
        }

        builder.set_format(self.core.config.format.clone());
        builder.populate_bg_layouts(layouts);

        self.writer_tracker.pipelines.insert(key.clone());
        match mode {
            InitMode::Immediate => {
                let _ = self.pipeline_handler.request_wait(&key, builder);
            },
            InitMode::Deferred => {
                self.pipeline_handler.request_new(&key, builder);
            }
        }
    }

    fn init_bind_group(&mut self, group_id: &String, bind_keys: Vec<ResourceID>) {
        if !self.layout_handler.is_ready(group_id) {
            return;
        }

        let mut resource_pairs = Vec::with_capacity(bind_keys.len());
        for key in &bind_keys {
            if let Some(buffer) = self.buffer_handler.get(&key) {
                resource_pairs.push((key.clone(), BindGroupResource::Buffer(Arc::clone(buffer))))
            }
            // add texture check when ready
        }

        // all resources found, safe to create bind group
        if resource_pairs.len() == bind_keys.len() {
            let layout = self.layout_handler.get(group_id).unwrap();
            let builder = BindGroupBuilder::new(Arc::clone(layout))
                .with_label(&group_id)
                .with_resources(resource_pairs);

            self.writer_tracker.bind_groups.insert(group_id.clone());
            self.bindgroup_handler.request_new(group_id, &builder);
        } 
        // else {
        //     panic!("One or more resources for bind group {} are missing!", group_id);
        // }
    }

    /// Prepare the context for the next frame
    pub fn prepare_next_frame(&mut self, mut old_reader: ResourceTracker) -> ResourceTracker {
        self.sync_handlers();

        self.core.window.request_redraw();

        old_reader.clear();
        old_reader.copy_from(&self.writer_tracker);

        std::mem::swap(&mut self.writer_tracker, &mut old_reader);

        old_reader
    } 

    // sync the handlers
    pub fn sync_handlers(&mut self) {
        self.layout_handler.sync();
        self.mesh_handler.sync();
        self.pipeline_handler.sync();
        self.bindgroup_handler.sync();
        self.buffer_handler.sync();
    }

    /// resize the surface that the context renders to (also resizes the window)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.core.resize(width, height);
    }

    /// Create resources from a list of create commands
    pub fn create_resources(&mut self, create_cmds: &Vec<CreateCommand>) {
        self.sync_handlers();
        for create_cmd in create_cmds {
            match create_cmd {
                CreateCommand::Mesh { id, builder } => {
                    self.writer_tracker.meshes.insert(id.clone());
                    self.mesh_handler.request_new(id, builder);
                },
                CreateCommand::Buffer { id, builder } => {
                    self.writer_tracker.buffers.insert(id.clone());
                    self.buffer_handler.request_new(&id, builder);
                },
                CreateCommand::BindGroupLayout { id, builder } => {
                    self.writer_tracker.bg_layouts.insert(id.clone());
                    self.layout_handler.request_new(&id, builder);
                }
                CreateCommand::RenderPipeline { id, builder } => {
                    self.init_pipeline(&id, &mut builder.clone(), InitMode::Deferred);
                },
                CreateCommand::BindGroup { id, bind_keys } => {
                    self.init_bind_group(&id, bind_keys.clone());
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

        // verify camera existance
        let camera_group = match self.bindgroup_handler.get(&renderer.get_camera_key()) {
            Some(data) => data,
            None => return Ok(()) // if the camera buffer is not ready, we can't draw anything
        };

        // prepare output and render pass
        let output = self.core.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.core.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

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
            self.draw_meshes(&renderer, &mut render_pass);
        }

        self.core.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// draw meshes to the current texture using the provided render pass
    fn draw_meshes(&mut self, renderer: &Renderer, render_pass: &mut wgpu::RenderPass) {
        let mut missing_meshes: Vec<u32> = Vec::new();
        let mut missing_pipelines: Vec<String> = Vec::new();

        for draw_cmd in renderer.draw_cmds() {
            if self.mesh_handler.status_of(&draw_cmd.id).is_none() {
                missing_meshes.push(draw_cmd.id);
            }
            if self.pipeline_handler.status_of(&draw_cmd.mat_id).is_none() {
                missing_pipelines.push(draw_cmd.mat_id.clone());
            }
        }

        for draw_cmd in renderer.draw_cmds() {
            if missing_meshes.contains(&draw_cmd.id) || missing_pipelines.contains(&draw_cmd.mat_id) {
                continue;
            }

            let m_status = self.mesh_handler.status_of(&draw_cmd.id);
            let p_status = self.pipeline_handler.status_of(&draw_cmd.mat_id);
            let u_status = self.bindgroup_handler.status_of(&draw_cmd.mat_id);

            if let (Some(ResourceStatus::Ready(mesh)), 
                    Some(ResourceStatus::Ready(pipeline)), 
                    Some(ResourceStatus::Ready(uniforms))) = (m_status, p_status, u_status) 
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(MATERIAL_UNIFORMS, uniforms, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }
    }
}