use std::sync::Arc;

use crate::graphics::{
    camera::Camera, material::{Material, UniformEntry}, mesh::{Mesh, MeshData}, presets::RenderPipelineConfig, traits::ResourceDescriptor, u_buffer_handler::UniformBufferHandler
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

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
/// Uniforms that are global to the entire scene
pub struct GlobalUniforms {
    view_proj: [f32; 16],
    cam_pos: [f32; 3],
    elapsed_time: f32,
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
    elapsed_time: f32,
}

impl Renderer {
    pub fn new(elapsed_time: f32) -> Self {
        Self { 
            draw_cmds: Vec::new(),
            update_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK,
            camera_key: "".to_string(),
            elapsed_time
        }
    }

    /// set the camera for the current frame
    pub fn set_camera<C: Camera>(&mut self, camera: &mut C) {
        camera.update_view_proj_mat();

        let globals = GlobalUniforms {
            view_proj: camera.get_view_proj_mat().to_cols_array(),
            cam_pos: camera.get_position().to_array(),
            elapsed_time: self.elapsed_time,
        };

        self.update_cmds.push(UpdateCommand { 
            uniform_id: camera.get_layout_id(), 
            entries: vec![UniformEntry {
                bind_slot: 0,
                data: UniformBufferHandler::pad_uniform(globals) 
            }],
        });

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
    pub fn draw<M: Material + Clone>(&mut self, mesh: &mut Mesh<M>) {
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
