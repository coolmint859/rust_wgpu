use std::{collections::HashSet, sync::Arc};

use crate::graphics::{
    bind_group_layout::BindGroupLayoutBuilder, camera::Camera, gpu_resource::ResourceBuilder, material::Material, mesh::{Mesh, MeshData}, render_pipeline::RenderPipelineBuilder, uniform::{BindGroupBuilder, UniformEntry}
};

pub struct LayoutCommand {
    pub layout_id: String,
    pub layout_builder: BindGroupLayoutBuilder
}

/// commands for drawing a mesh to the screen
#[derive(Clone, Debug)]
pub struct DrawCommand {
    pub mesh_id: u32,
    pub material_id: String, 
    pub data: Arc<MeshData>, 
    pub rpip_builder: RenderPipelineBuilder,
    pub z_depth: f32,
}

/// Commands for updating buffers
#[derive(Clone, Debug)]
pub struct UpdateCommand {
    pub uniform_id: String,
    pub entries: Vec<UniformEntry>
}

/// Uniforms that are global to the entire scene
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
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
    submitted_layouts: HashSet<String>,
    layout_cmds: Vec<LayoutCommand>,
    draw_cmds: Vec<DrawCommand>,
    update_cmds: Vec<UpdateCommand>,
    clear_color: wgpu::Color,
    camera_key: String,
    elapsed_time: f32,
}

impl Renderer {
    pub fn new(elapsed_time: f32) -> Self {
        Self {
            submitted_layouts: HashSet::new(),
            layout_cmds: Vec::new(),
            draw_cmds: Vec::new(),
            update_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK,
            camera_key: "".to_string(),
            elapsed_time
        }
    }

    /// Get the draw commands from this renderer
    pub fn draw_cmds(&self) -> &Vec<DrawCommand> {
        &self.draw_cmds
    }

    /// Get the update commands from this renderer
    pub fn update_cmds(&self) -> &Vec<UpdateCommand> {
        &self.update_cmds
    }

    pub fn layout_cmds(&self) -> &Vec<LayoutCommand> {
        &self.layout_cmds
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

    /// set the camera for the current frame
    pub fn set_camera<C: Camera>(&mut self, camera: &mut C) {
        if camera.is_dirty() {
            camera.update_view_proj_mat();

            let camera_key = camera.get_layout_id();
            self.request_layout(&camera_key, camera.get_layout_builder());

            let globals = GlobalUniforms {
                view_proj: camera.get_view_proj_mat().to_cols_array(),
                cam_pos: camera.get_position().to_array(),
                elapsed_time: self.elapsed_time,
            };

            self.update_cmds.push(UpdateCommand { 
                uniform_id: camera_key.clone(), 
                entries: vec![UniformEntry {
                    bind_slot: 0,
                    data: BindGroupBuilder::pad_uniform(globals) 
                }],
            });
        }

        self.camera_key = camera.get_layout_id();
    }

    /// Draw an object to the current texture
    pub fn draw<M: Material>(&mut self, mesh: &mut Mesh<M>) {
        self.request_layout(&mesh.material.get_key(),  mesh.material.get_layout_builder());

        if mesh.to_updated() {
            self.update_cmds.push(
                UpdateCommand { 
                    uniform_id: mesh.material.get_key(), 
                    entries: mesh.get_data()
                }
            )
        }

        self.draw_cmds.push(
            DrawCommand {
                mesh_id: mesh.data.get_key(),
                material_id: mesh.material.get_key(),
                data: Arc::clone(&mesh.data), 
                rpip_builder: mesh.pipeline.clone(),
                z_depth: mesh.transform.position.z,
            }
        );
    }

    /// Request a layout command to be queued. Commands with the same key already queued will be skipped
    fn request_layout(&mut self, id: &String, builder: BindGroupLayoutBuilder) {
        if self.submitted_layouts.insert(id.clone()) {
            self.layout_cmds.push(LayoutCommand { 
                layout_id: id.clone(), 
                layout_builder: builder
            });
        }
    }
}
