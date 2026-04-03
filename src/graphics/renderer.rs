use std::sync::Arc;

use crate::graphics::{
    camera::Camera, material::{Material, UniformEntry}, mesh::{Mesh, MeshData}, presets::RenderPipelineConfig, traits::ResourceDescriptor
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
    pub uniform_id: String,
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
    camera_key: String,
}

impl Renderer {
    pub fn new() -> Self {
        Self { 
            draw_cmds: Vec::new(),
            update_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK,
            camera_key: "".to_string(),
        }
    }

    /// set the camera for the current frame
    pub fn set_camera<C: Camera>(&mut self, camera: &mut C) {
        if camera.is_dirty() {
            camera.update_view_proj_mat();

            self.update_cmds.push(UpdateCommand { 
                uniform_id: camera.get_layout_id(), 
                entries: camera.get_data(),
            })
        }

        self.camera_key = camera.get_layout_id();
    }

    /// Get the currently set camera's key
    pub fn get_camera_key(&self) -> String {
        self.camera_key.clone()
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
    pub fn draw<M: Material>(&mut self, mesh: &mut Mesh<M>) {
        let transform_dirty = mesh.transform.is_dirty();
        let material_dirty = mesh.material.is_dirty();

        // only create an update command if the material or transform have changed
        if transform_dirty || material_dirty {
            mesh.transform.update_world_mat(); // make sure world matrix is up to date

            let model_mat = mesh.transform.world_matrix();
            let uniform_entries = mesh.material.get_data(model_mat);

            self.update_cmds.push(
                UpdateCommand { 
                    uniform_id: mesh.material.get_key(), 
                    entries: uniform_entries
                }
            )
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
