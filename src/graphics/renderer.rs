use std::sync::Arc;

use crate::graphics::{
    material::{Material, UniformEntry}, 
    mesh::{Mesh, MeshData}, 
    presets::RenderPipelineConfig, traits::ResourceDescriptor,
};

#[derive(Clone, Debug)]
pub struct DrawCommand {
    pub mesh_id: u32,
    pub material_id: String, 
    pub data: Arc<MeshData>, 
    pub rpip_config: RenderPipelineConfig
}

#[derive(Clone, Debug)]
pub struct UpdateCommand {
    pub material_id: String,
    pub entries: Vec<UniformEntry>
}

/// Constructs render commands from mesh and material data.
/// 
/// This acts as a translator for high level constructs into low level data 
/// for the WgpuContext during a single frame.
pub struct Renderer {
    draw_cmds: Vec<DrawCommand>,
    update_cmds: Vec<UpdateCommand>,
    clear_color: wgpu::Color,
}

impl Renderer {
    pub fn new() -> Self {
        Self { 
            draw_cmds: Vec::new(),
            update_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK
        }
    }

    // Set the background color for the frame
    pub fn set_bg_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = wgpu::Color { r, g, b, a: 1.0 }
    }

    // Get the currently set background color (default is black)
    pub fn get_bg_color(&self) -> &wgpu::Color {
        &self.clear_color
    }

    /// Draw an object to the current texture
    pub fn draw<M: Material>(&mut self, mesh: &Mesh<M>) {
        let mut uniform_entries = Vec::new();
        if mesh.transform.is_dirty() {
            uniform_entries.push(UniformEntry {
                bind_slot: 0,
                data: mesh.transform.as_byte_vec()
            })
        }

        if let Some(uniform_data) = mesh.material.diff() {
            uniform_entries.extend(uniform_data);
        }

        // only create update command if transform or material data have changed
        if uniform_entries.len() > 0 {
            self.update_cmds.push(
                UpdateCommand {
                    material_id: mesh.material.get_key(),
                    entries: uniform_entries
                }
            );
        }

        self.draw_cmds.push(
            DrawCommand {
                mesh_id: mesh.data.get_key().clone(),
                material_id: mesh.material.get_key(),
                data: Arc::clone(&mesh.data), 
                rpip_config: mesh.pipeline.clone()
            }
        );
    }

    /// Get the draw commands from this renderer
    pub fn draw_cmds(&self) -> Vec<DrawCommand> {
        self.draw_cmds.to_vec()
    }

    /// Get the update commands from this renderer
    pub fn update_cmds(&self) -> Vec<UpdateCommand> {
        self.update_cmds.to_vec()
    }
}
