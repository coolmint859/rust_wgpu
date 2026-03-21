#![allow(dead_code)]
use anyhow::Ok;
use wgpu::util::DeviceExt;
use winit::window::Window;
use std::sync::Arc;
use bytemuck;

use super::renderer::Renderer;
use super::vertex::Vertex;
use super::pipeline;
use super::buffer;
use super::core::WgpuCore;
use super::mesh::Mesh;

/// Represents the entire WebGPU rendering context
pub struct WgpuContext {
    core: WgpuCore,
    pipeline_handler: pipeline::PipelineHandler,
    buffer_handler: buffer::BufferHandler,

    mesh: Mesh,
}

impl WgpuContext {
    const VERT_MAIN: &str = "vs_main";
    const FRAG_MAIN: &str = "fs_main";

    pub async fn new(window: Arc<Window>) -> Self {
        let core = WgpuCore::new(window).await;

        let mut pipeline_handler = pipeline::PipelineHandler::new(
            &core.device,
            core.config.format.clone()
        );

        pipeline_handler.request_pipeline(
            pipeline::ShaderConfig {
                name: String::from("shader"),
                path: String::from("assets/shaders/shader.wgsl"),
                vert_main: String::from("vs_main"),
                frag_main: String::from("fs_main"),
            }
        );

        let mesh = Mesh::new(
            vec![
                Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
                Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
                Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
                Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
                Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E
            ],
            vec![
                0, 1, 4, 
                1, 2, 4,
                2, 3, 4
            ]
        );

        let mut buffer_handler = buffer::BufferHandler::new(&core.device);
        buffer_handler.request_gpu_mesh(&mesh);

        Self {
            core,
            pipeline_handler,
            buffer_handler,
            mesh
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.core.resize(width, height);
    }

    pub fn render(&mut self, mut _renderer: Renderer) -> anyhow::Result<()> {
        self.core.window.request_redraw();
        self.pipeline_handler.check_ready_pipelines();
        self.buffer_handler.check_ready_buffers();

        if !self.core.is_surface_configured() {
            return Ok(());
        }

        let pipeline = match self.pipeline_handler.get_pipeline("shader") {
            Some(pipeline) => pipeline,
            None => return Ok(())
        };

        let gpu_mesh = match self.buffer_handler.get_gpu_mesh(self.mesh.id()) {
            Some(gpu_mesh) => gpu_mesh,
            None => return Ok(())
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
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1, g: 0.2, b: 0.3, a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        }
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                }
            );

            render_pass.set_pipeline(pipeline);
            render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(gpu_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..gpu_mesh.num_indices, 0, 0..1);
        }

        self.core.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}