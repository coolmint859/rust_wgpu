#![allow(dead_code)]
use winit::window::Window;
use std::sync::Arc;

use super::{
    transient::*,
    pipeline::{ RenderPipelineHandler },
    traits::{ Handler, ResourceDescriptor, CommandBuffer },
    mesh_buffer,
    core::WgpuCore,
};

/// Represents the entire WebGPU rendering context
pub struct WgpuContext {
    core: WgpuCore,
    pipeline_handler: RenderPipelineHandler,
    mesh_buffer_handler: mesh_buffer::MeshBufferHandler,
}

impl WgpuContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let core = WgpuCore::new(window).await;

        let pipeline_handler = RenderPipelineHandler::new(
            &core.device,
            core.config.format.clone()
        );

        let buffer_handler = mesh_buffer::MeshBufferHandler::new(&core.device);

        Self {
            core,
            pipeline_handler,
            mesh_buffer_handler: buffer_handler,
        }
    }

    /// initialize resources prior to rendering state
    pub fn init_resources(&mut self, init_state: StateInit) {
        for command in init_state.get_commands() {
            match command {
                InitCommand::Mesh(mesh) => {
                    self.mesh_buffer_handler.request_wait(Arc::clone(&mesh.data))
                },
                InitCommand::Pipeline(pip_config) => {
                    self.pipeline_handler.request_wait(pip_config)
                },
            }
        }
    }

    /// prepare the context for the next frame
    pub fn prepare_next_frame(&mut self) {
        self.pipeline_handler.sync();
        self.mesh_buffer_handler.sync();

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

            for cmd in renderer.get_commands() {
                match cmd {
                    RenderCommand::Mesh(mesh, pip_config) => {
                        let pipeline = match self.pipeline_handler.get(&pip_config.name) {
                            Some(pipeline) => pipeline,
                            None => {
                                self.pipeline_handler.request_new(Arc::clone(&pip_config));
                                continue;
                            }
                        };

                        let gpu_mesh = match self.mesh_buffer_handler.get(&mesh.data.get_key()) {
                            Some(gpu_mesh) => gpu_mesh,
                            None => {
                                self.mesh_buffer_handler.request_new(Arc::clone(&mesh.data));
                                continue;
                            }
                        };

                        render_pass.set_pipeline(pipeline);
                        render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
                        render_pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                        render_pass.draw_indexed(0..gpu_mesh.num_indices, 0, 0..1);
                    }
                }
            }
        }

        self.core.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}