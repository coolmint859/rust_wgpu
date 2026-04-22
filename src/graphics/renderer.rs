#![allow(dead_code)]
use std::{collections::HashSet, sync::Arc};

use glam::Mat4;

use crate::graphics::{entity::Entity, texture::SamplerBuilder, wpgu_context::{ResourceBinding, ResourceID, ResourceScope}};

use super::{
    bind_group::*, 
    buffer::BufferBuilder, 
    camera::Camera, 
    init_state::InitMode, 
    material::UniformBuilder, 
    mesh::MeshData,
    render_pipeline::RenderPipelineBuilder, 
    texture::TextureBuilder, 
    tracker::ResourceTracker,
    wpgu_context::{GLOBAL_UNIFORMS, INSTANCE_UNIFORMS, MATERIAL_UNIFORMS}
};

/// Commands used for creating resources
#[derive(Clone, Debug)]
pub enum CreateCommand {
    BindGroupLayout{ builder: BindGroupLayoutBuilder },
    BindGroup{ id: String, layout_id: BindGroupLayoutBuilder, resource_ids: Vec<ResourceBinding> }, // bind group builders are made by the context
    RenderPipeline{ builder: RenderPipelineBuilder, mode: InitMode },
    Mesh { id: u32, builder: Arc<MeshData> },
    Buffer { id: ResourceID, builder: BufferBuilder },
    Texture { id: ResourceID, builder: TextureBuilder },
    Sampler { id: ResourceID, builder: SamplerBuilder },
}

/// Command used to update uniform buffers
#[derive(Clone, Debug)]
pub struct UpdateCommand {
    pub key: ResourceID, 
    pub data: Vec<u8> 
}

/// Command used to draw a single instance of a mesh to the current texture
#[derive(Clone, Debug)]
pub struct DrawCommand {
    /// used to get the vertex/index buffers
    pub mesh_id: u32,
    /// used to get the mesh's bind group
    pub entity_group: String,
    /// used to get the material's bind group
    pub material_group: String,
    /// used to get the render pipeline
    pub rpip_id: RenderPipelineBuilder,
    /// used for z-ordering
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

// Uniforms that belong to an entity instance
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EntityUniforms {
    model_mat: Mat4
}

/// Constructs render commands from mesh and material data.
/// 
/// This acts as a translator for high level constructs into low level data 
/// for the WgpuContext during a single frame.
pub struct Renderer {
    submitted_layouts: HashSet<BindGroupLayoutBuilder>,

    // commands
    create_cmds: Vec<CreateCommand>,
    draw_cmds: Vec<DrawCommand>,
    update_cmds: Vec<UpdateCommand>,

    // general frame settings
    clear_color: wgpu::Color,
    globals_id: ResourceID,
    globals_layout: Option<BindGroupLayoutBuilder>,
    elapsed_time: f32,

    // tracker for preventing unnecessary command generation
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
            globals_id: ResourceID { key: "".to_string(), scope: ResourceScope::Global },
            globals_layout: None,
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
        self.globals_id.key.clone()
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
        camera.update();

        let camera_id = ResourceID { 
            key: camera.get_key(), 
            scope: ResourceScope::Global 
        };
        let camera_group = ResourceBinding {
            id: camera_id.clone(),
            slot: 0
        };

        self.request_layout(camera.get_layout_builder());
        self.request_global_uniforms(camera);
        self.request_bind_group(&camera_id.key, camera.get_layout_builder(), vec![camera_group]);

        self.globals_id = camera_id;
        self.globals_layout = Some(camera.get_layout_builder());
    }

    /// request/create global uniform data from the current camera
    fn request_global_uniforms<C: Camera>(&mut self, camera: &mut C) {
        let camera_id = camera.get_key();

        let globals = GlobalUniforms {
            view_proj: camera.get_view_proj_mat().to_cols_array(),
            cam_pos: camera.get_position().to_array(),
            elapsed_time: self.elapsed_time,
        };

        let builder = BufferBuilder::as_uniform(0)
            .with_label(&camera_id)
            .with_data_from_struct(globals);

        // if a buffer wasn't just requested, issue an update command
        let key = ResourceID { key: camera_id.clone(), scope: ResourceScope::Global };
        if !self.request_buffer(&key, builder) {
            self.update_cmds.push(UpdateCommand { 
                key: key.clone(), data: BufferBuilder::to_padded_vec(globals),
            });
        }
    }

    /// Draw an object using the given transform and material to the current texture
    /// 
    /// TODO: untangle layout key from bind group key to allow similar materials to share layouts
    pub fn draw(&mut self, entity: &mut Entity) {        
        let mut pipeline = entity.pipeline.clone();
        pipeline.add_bg_layout(GLOBAL_UNIFORMS, self.globals_layout.as_mut().unwrap().clone());
        
        self.request_mesh(entity.mesh.get_data_builder());
        self.process_transform(entity, &mut pipeline);
        self.process_uniforms(entity, &mut pipeline);

        self.request_render_pipeline(pipeline.clone());

        self.draw_cmds.push(
            DrawCommand {
                mesh_id: entity.mesh.get_data_key(),
                entity_group: entity.transform_id().key,
                material_group: entity.get_namespace_id().key,
                rpip_id: pipeline,
                z_depth: entity.transform.position.z,
            }
        );
    }

    /// Request an entity's transform layout and buffer 
    fn process_transform(&mut self, entity: &mut Entity, pipeline: &mut RenderPipelineBuilder) {
        let entity_id = entity.transform_id();
        let entity_layout = entity.transform_layout();

        self.request_layout(entity_layout.clone());
        pipeline.add_bg_layout(INSTANCE_UNIFORMS, entity_layout.clone());
        self.request_bind_group(&entity_id.key, entity_layout, vec![entity.transform_binding()]);

        // update buffers if transform had changed (is guarenteed to on first frame)
        if entity.transform.update() {
            let entity_uniforms = EntityUniforms {
                model_mat: entity.transform.world_matrix()
            };
            let uniform_builder = BufferBuilder::as_uniform(0)
                .with_label(&entity_id.key)
                .with_data_from_struct(entity_uniforms.clone());
            self.request_buffer(&entity_id, uniform_builder);

            let data = BufferBuilder::to_padded_vec(entity_uniforms);
            self.update_cmds.push(UpdateCommand { key: entity_id.clone(), data });
        }
    }

    /// Process an entity's material uniforms
    fn process_uniforms(&mut self, entity: &mut Entity, pipeline: &mut RenderPipelineBuilder) {
        fn namespace_id(entity: &Entity, resource_key: &String) -> String {
            format!("{}::{}", entity.mesh.get_key(), resource_key)
        }

        let mut bindings: Vec<ResourceBinding> = Vec::new();
        for (mut binding, u_builder_enum) in entity.material.get_uniforms() {
            let mut uniform_id = binding.id.clone();
            match binding.id.scope {
                ResourceScope::Entity => {
                    uniform_id.key = namespace_id(entity, &uniform_id.key);
                }
                _ => {}
            };

            match u_builder_enum {
                UniformBuilder::Buffer(builder) => {
                    self.request_buffer(&uniform_id, builder);
                }
                UniformBuilder::Texture(builder) => {
                    self.request_texture(&uniform_id, builder);
                }
                UniformBuilder::Sampler(builder) => {
                    self.request_sampler(&uniform_id, builder);
                }
            }

            binding.id = uniform_id;
            bindings.push(binding);
        }

        // request complete layout and add to pipeline
        let material_layout = entity.material.get_layout();
        self.request_layout(material_layout.clone());
        self.request_bind_group(&entity.get_namespace_id().key, material_layout.clone(), bindings);
        pipeline.add_bg_layout(MATERIAL_UNIFORMS,material_layout);

        for (mut uniform_id, data) in entity.material.get_buffers_updated() {
            uniform_id.key = namespace_id(entity, &uniform_id.key);

            self.update_cmds.push(UpdateCommand { key: uniform_id, data });
        }
    }

    /// Request a layout command to be queued. Commands with the same key already queued will be skipped
    fn request_layout(&mut self, builder: BindGroupLayoutBuilder) {
        let not_submitted = self.submitted_layouts.insert(builder.clone());
        let not_requested = !self.tracker.as_mut().unwrap().bg_layouts.contains(&builder); 
        
        if not_submitted && not_requested {
            self.create_cmds.push(CreateCommand::BindGroupLayout { builder });
        }
    }

    /// request a create buffer command to be queued. Commands with the same key already queued will be skipped.
    fn request_buffer(&mut self, key: &ResourceID, builder: BufferBuilder) -> bool {
        if !self.tracker.as_mut().unwrap().buffers.contains(key) {
            self.create_cmds.push(CreateCommand::Buffer { id: key.clone(), builder });
            return true;
        }
        return false
    }

    /// request a create texture command to be queued. Commands with the same key already queued will be skipped.
    fn request_texture(&mut self, key: &ResourceID, builder: TextureBuilder) {
        if !self.tracker.as_mut().unwrap().textures.contains(key) {
            self.create_cmds.push(CreateCommand::Texture { id: key.clone(), builder });
        }
    }

    /// Request a create texture command to be queued. Commands with the same key already queued will be skipped.
    fn request_sampler(&mut self, key: &ResourceID, builder: SamplerBuilder) {
        if !self.tracker.as_mut().unwrap().samplers.contains(key) {
            self.create_cmds.push(CreateCommand::Sampler { id: key.clone(), builder });
        }
    }

    /// request a create mesh command to be queued. Commands with the same key already queued will be skipped.
    fn request_mesh(&mut self, mesh: Arc<MeshData>) {
        let mesh_id = mesh.id();
        if !self.tracker.as_mut().unwrap().meshes.contains(&mesh_id) {
            self.create_cmds.push(CreateCommand::Mesh { 
                id: mesh_id, 
                builder: mesh
            });
        }
    }

    /// request a create render pipeline command to be queued. Commands with the same key already queued will be skipped.
    fn request_render_pipeline(&mut self, pipeline: RenderPipelineBuilder) {
        if !self.tracker.as_mut().unwrap().pipelines.contains(&pipeline) {
            self.create_cmds.push(CreateCommand::RenderPipeline {
                builder: pipeline, mode: InitMode::Deferred
            });
        }
    }

    /// request a create bind group command to be queued. Commands with the same key already queued will be skipped.
    fn request_bind_group(&mut self, key: &String, layout_id: BindGroupLayoutBuilder, resource_ids: Vec<ResourceBinding>) {
        if !self.tracker.as_mut().unwrap().bind_groups.contains(key) {
            self.create_cmds.push(CreateCommand::BindGroup { 
                id: key.clone(),
                layout_id,
                resource_ids,
            });
        }
    }

    /// Take ownership of the renderer's tracker. Should only be called after all commands are recorded.
    pub(crate) fn take_tracker(&mut self) -> ResourceTracker {
        self.tracker.take().expect("Tracker already taken!")
    }
}
