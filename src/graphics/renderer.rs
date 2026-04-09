use std::{collections::HashSet, sync::Arc};

use crate::graphics::{
    bind_group::*, 
    buffer::BufferBuilder, 
    camera::Camera,
    material::Material, 
    mesh::{Mesh, MeshData}, 
    render_pipeline::RenderPipelineBuilder,
    tracker::ResourceTracker
};

/// Commands for creating resources
/// 
#[derive(Clone, Debug)]
pub enum CreateCommand {
    BindGroupLayout{ id: String, builder: BindGroupLayoutBuilder },
    BindGroup{ id: String, bind_keys: Vec<ResourceID> }, // bind group builders are made by the context
    RenderPipeline{ id: String, builder: RenderPipelineBuilder },
    Mesh { id: u32, builder: Arc<MeshData> },
    Buffer { id: ResourceID, builder: BufferBuilder}
}

#[derive(Clone, Debug)]
pub struct UpdateCommand {
    pub key: ResourceID, 
    pub data: Vec<u8> 
}

#[derive(Clone, Debug)]
pub struct DrawCommand {
    pub id: u32,
    pub mat_id: String,
    pub data: Arc<MeshData>,
    pub rpip_builder: RenderPipelineBuilder,
    pub z_depth: f32,
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

    create_cmds: Vec<CreateCommand>,
    draw_cmds: Vec<DrawCommand>,
    update_cmds: Vec<UpdateCommand>,

    clear_color: wgpu::Color,
    camera_key: String,
    elapsed_time: f32,

    tracker: Option<ResourceTracker>,
}

impl Renderer {
    pub fn new(tracker: ResourceTracker, elapsed_time: f32) -> Self {
        Self {
            submitted_layouts: HashSet::new(),
            create_cmds: Vec::new(),
            draw_cmds: Vec::new(),
            update_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK,
            camera_key: "".to_string(),
            elapsed_time,
            tracker: Some(tracker)
        }
    }

    /// Clear all commands in the queues
    pub fn clear_commands(&mut self) {
        self.create_cmds.clear();
        self.update_cmds.clear();
        self.draw_cmds.clear();
    }

    /// Get the draw commands from this renderer
    pub fn draw_cmds(&self) -> &Vec<DrawCommand> {
        &self.draw_cmds
    }

    /// Get the update commands from this renderer
    pub fn update_cmds(&self) -> &Vec<UpdateCommand> {
        &self.update_cmds
    }

    /// Get the create commands from this render
    pub fn create_cmds(&self) -> &Vec<CreateCommand> {
        &self.create_cmds
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
        let camera_key = camera.get_layout_id();
        self.request_layout(&camera_key, || {camera.get_layout_builder()});

        if camera.is_dirty() {
            camera.update_view_proj_mat();

            let globals = GlobalUniforms {
                view_proj: camera.get_view_proj_mat().to_cols_array(),
                cam_pos: camera.get_position().to_array(),
                elapsed_time: self.elapsed_time,
            };

            let key = ResourceID { group_id: camera_key.clone(), binding: 0};
            let builder_fn = || {
                BufferBuilder::as_uniform(0)
                    .with_data_from_struct(globals)
            };

            // only update if the buffer was already requested
            if !self.request_buffer(&key, builder_fn) {
                self.update_cmds.push(UpdateCommand { 
                    key: key.clone(), data: BufferBuilder::to_padded_vec(globals),
                });
            }

            self.request_bind_group(&camera_key, vec![key]);
        }

        self.camera_key = camera.get_layout_id();
    }

    /// Draw an object to the current texture
    pub fn draw<M: Material>(&mut self, mesh: &mut Mesh<M>) {
        let mesh_key = &mesh.material.get_key();
        self.request_layout(mesh_key, || { mesh.material.get_layout_builder() });

        if self.request_mesh(mesh) { return;} // GATE 1: mesh data must exist
        if self.process_buffers(mesh) { return; } // GATE 2: // buffers must exist

        // only request bind group/pipeline when all buffers have already been requested.
        let uniform_keys: Vec<ResourceID> = mesh.get_requirements().iter().map(|(key, _)| { key.clone() }).collect();
        let bind_group_requested = self.request_bind_group(&mesh.material.get_key(), uniform_keys);
        let pipeline_requested = self.request_render_pipeline(mesh);
        if bind_group_requested && pipeline_requested { return; } // GATE 3: pipeline/bind group must exist

        // draw command issued only after bind group, pipeline, and buffers already were requested
        self.draw_cmds.push(
            DrawCommand {
                id: mesh.data.id(),
                mat_id: mesh.material.get_key(),
                data: Arc::clone(&mesh.data), 
                rpip_builder: mesh.pipeline.clone(),
                z_depth: mesh.transform.position.z,
            }
        );
    }

    /// Process a mesh's buffers
    fn process_buffers<M: Material>(&mut self, mesh: &mut Mesh<M>) -> bool {
        let most_recent_data = mesh.get_updated();
        let requirements = mesh.get_requirements();

        let mut buffer_request_made = false;
        for (id, builder) in requirements {
            buffer_request_made = self.request_buffer(&id, || { builder }) || buffer_request_made;
        }
        if buffer_request_made { return true; } // buffers must exist

        for (id, data) in most_recent_data {
            self.update_cmds.push(UpdateCommand { key: id, data });
        }

        return false;
    }

    /// Request a layout command to be queued. Commands with the same key already queued will be skipped
    fn request_layout(&mut self, key: &String, builder_fn: impl FnOnce() -> BindGroupLayoutBuilder) {
        let not_submitted = self.submitted_layouts.insert(key.clone());
        let not_requested = !self.tracker.as_mut().unwrap().bg_layouts.contains(key); 
        
        if not_submitted && not_requested {
            let builder = builder_fn();
            self.create_cmds.push(CreateCommand::BindGroupLayout { id: key.clone(), builder });
        }
    }

    /// request a create buffer command to be queued. Commands with the same key already queued will be skipped.
    fn request_buffer(&mut self, key: &ResourceID, builder_fn: impl FnOnce() -> BufferBuilder) -> bool {
        if !self.tracker.as_mut().unwrap().buffers.contains(&key) {
            let builder = builder_fn();
            self.create_cmds.push(CreateCommand::Buffer { id: key.clone(), builder });
            return true;
        }
        return false;
    }

    /// request a create mesh command to be queued. Commands with the same key already queued will be skipped.
    fn request_mesh<M: Material>(&mut self, mesh: &Mesh<M>) -> bool {
        let mesh_id = mesh.data.id();
        if !self.tracker.as_mut().unwrap().meshes.contains(&mesh_id) {
            self.create_cmds.push(CreateCommand::Mesh { 
                id: mesh_id, 
                builder: mesh.get_data_builder()
            });
            return true;
        }
        return false;
    }

    /// request a create render pipeline command to be queued. Commands with the same key already queued will be skipped.
    fn request_render_pipeline<M: Material>(&mut self, mesh: &Mesh<M>) -> bool {
        let key = mesh.material.get_key();
        if !self.tracker.as_mut().unwrap().pipelines.contains(&key) {
            self.create_cmds.push(CreateCommand::RenderPipeline { 
                id: key, 
                builder: mesh.get_pipeline_builder()
            });
            return true;
        }
        return false;
    }

    /// request a create bind group command to be queued. Commands with the same key already queued will be skipped.
    fn request_bind_group(&mut self, key: &String, resource_keys: Vec<ResourceID>) -> bool {
        if !self.tracker.as_mut().unwrap().bind_groups.contains(key) {
            self.create_cmds.push(CreateCommand::BindGroup { 
                id: key.clone(), 
                bind_keys: resource_keys
            });
            return true;
        }
        return false;
    }

    /// Take ownership of the renderer's tracker. Should only be called after all commands are recorded.
    pub(crate) fn take_tracker(&mut self) -> ResourceTracker {
        self.tracker.take().expect("Tracker already taken!")
    }
}
