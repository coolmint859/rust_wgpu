#![allow(dead_code)]
use winit::window::Window;
use std::sync::Arc;

use crate::graphics::{
    bind_group_layout::LayoutHandler, 
    core::WgpuCore, 
    gpu_resource::{GpuResourceHandler, ResourceBuilder, ResourceStatus}, 
    init_state::{InitMode, StateInit}, 
    material::UniformGroup, 
    mesh_buffer::MeshBufferHandler, 
    presets::BindingLayout, 
    render_pipeline::RenderPipelineBuilder, 
    renderer::Renderer, 
    traits::Handler, 
    uniform_buffer::UniformBufferHandler
};

/// group binding number for global uniforms
pub const GLOBAL_UNIFORMS: u32 = 0;
/// group binding number for material uniforms
pub const MATERIAL_UNIFORMS: u32 = 1;

/// Represents the entire WebGPU rendering context
pub struct WgpuContext {
    core: WgpuCore,
    layout_handler: LayoutHandler,
    mesh_handler: MeshBufferHandler,
    uniform_handler: UniformBufferHandler,
    pipeline_handler: GpuResourceHandler<String, wgpu::RenderPipeline>,
}

impl WgpuContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let core = WgpuCore::new(window).await;

        let mut layout_handler = LayoutHandler::new(Arc::clone(&core.device));
        layout_handler.request_wait(BindingLayout::ColoredSprite.get());
        layout_handler.request_wait(BindingLayout::Camera2D.get());

        let mesh_handler = MeshBufferHandler::new(Arc::clone(&core.device));
        let uniform_handler = UniformBufferHandler::new(Arc::clone(&core.device));
        let pipeline_handler = GpuResourceHandler::new(Arc::clone(&core.device));

        Self {
            core,
            layout_handler,
            mesh_handler,
            uniform_handler,
            pipeline_handler,
        }
    }

    /// initialize resources prior to rendering state
    pub fn init_resources(&mut self, init_state: StateInit) {
        for rpip_cmd in init_state.get_rpip_cmds() {
            self.init_pipeline(rpip_cmd.builder, InitMode::Deferred);
        }
        
        for bgl_cmd in init_state.get_bgl_cmds() {
            match bgl_cmd.mode {
                InitMode::Immediate => {
                    self.layout_handler.request_wait(Arc::new(bgl_cmd.config));
                },
                InitMode::Deferred => {
                    self.layout_handler.request_new(Arc::new(bgl_cmd.config));
                }
            }
        }
    }

    fn init_pipeline(&mut self, mut builder: RenderPipelineBuilder, mode: InitMode) {
        let mut layouts = Vec::new();

        for id in &builder.get_layout_ids() {
            if let Some(layout) = self.layout_handler.get(&id) {
                layouts.push(Arc::clone(layout));
            } else {
                panic!("Required layout {} not found for pipeline {}", id, builder.get_key());
            }
        }

        builder.set_format(self.core.config.format.clone());
        builder.populate_bg_layouts(layouts);

        match mode {
            InitMode::Immediate => {
                self.pipeline_handler.request_wait(builder);
            },
            InitMode::Deferred => {
                self.pipeline_handler.request_new(builder);
            }
        }
    }

    /// Prepare the context for the next frame
    pub fn prepare_next_frame(&mut self) {
        self.layout_handler.sync();
        self.mesh_handler.sync();
        self.pipeline_handler.sync();
        self.uniform_handler.sync();

        self.core.window.request_redraw();
    } 

    /// resize the surface that the context renders to (also resizes the window)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.core.resize(width, height);
    }

    /// render commands given to the renderer instance
    pub fn render(&mut self, renderer: Renderer) -> anyhow::Result<()> {
        if !self.core.is_surface_configured() {
            return Ok(());
        }

        // UPDATE PASS: Update uniform buffers
        for cmd in &renderer.update_cmds() {
            if let Some(uniforms) = self.uniform_handler.get(&cmd.uniform_id) {
                for entry in &cmd.entries {
                    let buffer = uniforms.buffers.get(&entry.bind_slot).unwrap();

                    self.core.queue.write_buffer(buffer, 0, &entry.data);
                }
            } else if !self.uniform_handler.contains(&cmd.uniform_id) {
                self.uniform_handler.request_new(
                    Arc::new(UniformGroup {
                        key: cmd.uniform_id.clone(),
                        contents: cmd.entries.to_vec(),
                        bind_layout: Arc::clone(self.layout_handler.get(&cmd.uniform_id).unwrap())
                    })
                );
            }
        }

        // DRAW PASS: render meshes to screen

        // First check for camera existance
        let cam_data = match self.uniform_handler.get(&renderer.get_camera_key()) {
            Some(data) => data,
            None => return Ok(()) // if the camera buffer is not ready, we can't draw anything
        };

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

            // at this point we know the camera buffer exists, so we can safely bind it.
            render_pass.set_bind_group(GLOBAL_UNIFORMS, &cam_data.bind_group, &[]);

            let mut missing_meshes: Vec<u32> = Vec::new();
            let mut missing_pipelines: Vec<String> = Vec::new();

            for cmd in renderer.draw_cmds() {
                if self.mesh_handler.status_of(&cmd.mesh_id).is_none() {
                    self.mesh_handler.request_new(cmd.data);
                    missing_meshes.push(cmd.mesh_id);
                }
                if self.pipeline_handler.status_of(&cmd.material_id).is_none() {
                    self.init_pipeline(cmd.rpip_builder, InitMode::Deferred);

                    missing_pipelines.push(cmd.material_id);
                }
            }

            for cmd in renderer.draw_cmds() {
                if missing_meshes.contains(&cmd.mesh_id) || missing_pipelines.contains(&cmd.material_id) {
                    continue;
                }

                let m_status = self.mesh_handler.status_of(&cmd.mesh_id);
                let p_status = self.pipeline_handler.status_of(&cmd.material_id);
                let u_status = self.uniform_handler.status_of(&cmd.material_id);

                if let (Some(ResourceStatus::Ready(mesh)), 
                        Some(ResourceStatus::Ready(pipeline)), 
                        Some(ResourceStatus::Ready(uniforms))) = (m_status, p_status, u_status) 
                {
                    render_pass.set_pipeline(pipeline);
                    render_pass.set_bind_group(MATERIAL_UNIFORMS, &uniforms.bind_group, &[]);
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
                }
            }
        }

        self.core.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}