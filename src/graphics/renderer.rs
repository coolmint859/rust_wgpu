#![allow(dead_code)]
use std::{collections::HashSet, sync::Arc};

use crate::graphics::{entity::{Entity, EntityDesciptor, EntityInstances, RenderInfo}, geometry::GeometryBuilder, material::Material, presets::ShaderSpecPreset, shader::ShaderSpecBuilder, texture::SamplerBuilder, transform::Transform, wpgu_context::{ResourceBinding, ResourceID, ResourceScope}};

use super::{
    bind_group::*, 
    buffer::BufferBuilder, 
    camera::Camera, 
    init_state::InitMode, 
    material::UniformBuilder,
    render_pipeline::RenderPipelineBuilder, 
    texture::TextureBuilder, 
    tracker::ResourceTracker,
};

/// Commands used for creating resources
#[derive(Clone, Debug)]
pub enum CreateCommand {
    ShaderSpec { id: String, builder: ShaderSpecBuilder },
    BindGroup{ id: String, layout_id: BindGroupLayoutBuilder, bindings: Vec<ResourceBinding> },
    RenderPipeline{ shader_path: String, builder: RenderPipelineBuilder, mode: InitMode },
    Geometry { shader_path: String, key: String, builder: Arc<GeometryBuilder> },
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
    /// used to get the geometry buffers
    pub geometry_key: String,
    /// used to get the entity's bind group
    pub instance_key: String,
    /// used to get the material's bind group
    pub material_key: String,
    /// used to get the render pipeline and shader
    pub render_info: RenderInfo,
    /// used for z-ordering
    pub z_depth: f32,
}

/// Command used to draw multiple instances of a mesh to the current texture
#[derive(Clone, Debug)]
pub struct InstanceCommand {
    /// used to get the geometry buffers
    pub geometry_key: String,
    /// used to get the material's bind group
    pub material_key: String,
    /// used to get the transform buffer
    pub instance_key: ResourceID,
    /// used to get the render pipeline
    pub render_info: RenderInfo,
    /// the number of instances to draw,
    pub instances: u32,
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
    submitted_layouts: HashSet<BindGroupLayoutBuilder>,

    // commands
    create_cmds: Vec<CreateCommand>,
    draw_cmds: Vec<DrawCommand>,
    instance_cmds: Vec<InstanceCommand>,
    update_cmds: Vec<UpdateCommand>,

    // general frame settings
    clear_color: wgpu::Color,
    global_key: String,
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
            instance_cmds: Vec::new(),
            update_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK,
            global_key: "".to_string(),
            elapsed_time,
            tracker: Some(tracker)
        }
    }

    /// Clear all commands in the queues
    pub fn clear_commands(&mut self) {
        self.create_cmds.clear();
        self.update_cmds.clear();
        self.draw_cmds.clear();
        self.instance_cmds.clear();
    }

    /// Get the draw commands from this renderer
    pub fn draw_cmds(&self) -> &Vec<DrawCommand> {
        &self.draw_cmds
    }

    /// Get the draw instances command from this renderer
    pub fn instance_cmds(&self) -> &Vec<InstanceCommand> {
        &self.instance_cmds
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
        self.global_key.clone()
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

        let camera_id = camera.get_id();
        let globals = GlobalUniforms {
            view_proj: camera.get_view_proj_mat().to_cols_array(),
            cam_pos: camera.get_position().to_array(),
            elapsed_time: self.elapsed_time,
        };

        let builder = BufferBuilder::as_uniform()
            .with_label(&camera_id.key)
            .with_data_from_struct(globals);

        if !self.request_buffer(&camera_id, builder) {
            self.update_cmds.push(UpdateCommand { 
                key: camera_id.clone(), data: BufferBuilder::to_padded_vec(globals),
            });
        }
        
        self.request_bind_group( &camera_id.key, camera.get_layout_builder(), vec![camera.get_binding()]);
        
        self.global_key = camera_id.key;
    }

    /// Draw an entity to the current texture
    pub fn draw(&mut self, entity: &mut Entity) {
        let shader_path = entity.render_info.shader_path.clone();
        self.request_shader_spec(&shader_path);

        let entity_desc = entity.descriptor();
        self.request_geometry(&shader_path, &entity_desc.geometry_key, Arc::clone(&entity.geometry));
        self.process_transform(&entity_desc, &mut entity.transform);
        self.process_uniforms(&entity_desc, &entity.material);
        self.request_render_pipeline(&entity.render_info);

        self.draw_cmds.push(
            DrawCommand {
                geometry_key: entity_desc.geometry_key.clone(),
                instance_key: entity_desc.transform_id.key,
                material_key: entity_desc.namespace,
                render_info: entity.render_info.clone(),
                z_depth: entity.transform.get_position().z,
            }
        );
    }

    /// Draw many instances of an entity to the current texture
    pub fn draw_instances(&mut self, instances: &mut EntityInstances) {
        if instances.transforms.data.is_empty() { return; }

        let shader_path = instances.render_info.shader_path.clone();
        let entity_desc = instances.descriptor();
        self.request_shader_spec(&shader_path);

        self.request_geometry(&shader_path, &entity_desc.geometry_key, Arc::clone(&instances.geometry));
        self.process_instance_transforms(&entity_desc, &mut instances.transforms.data);
        self.process_uniforms(&entity_desc, &instances.material);

        self.request_render_pipeline(&instances.render_info);

        self.instance_cmds.push(
            InstanceCommand {
                geometry_key: entity_desc.geometry_key,
                instance_key: entity_desc.transform_id,
                material_key: entity_desc.namespace,
                render_info: instances.render_info.clone(),
                instances: instances.transforms.data.len() as u32
            }
        );
    }

    /// Request an entity's transform layout and buffer 
    fn process_transform(&mut self, entity_desc: &EntityDesciptor, transform: &mut Transform) {
        let tranform_id = entity_desc.transform_id.clone();

        let transform_group = entity_desc.transform_group.clone().unwrap();
        self.request_bind_group(&tranform_id.key, transform_group.layout, vec![transform_group.binding]);

        // update buffers if transform had changed (is guarenteed to on first frame)
        if transform.to_updated() {
            let matrix_data = bytemuck::bytes_of(&transform.world_matrix()).to_vec();

            let uniform_builder = BufferBuilder::as_uniform()
                .with_label(&tranform_id.key)
                .with_data(matrix_data.clone());

            self.request_buffer(&tranform_id, uniform_builder);
            self.update_cmds.push(UpdateCommand { key: tranform_id.clone(), data: matrix_data });
        }
    }

    /// Process transforms from multiple instances
    fn process_instance_transforms(&mut self, entity_desc: &EntityDesciptor, transforms: &mut Vec<Transform>) {
        let transform_id = entity_desc.transform_id.clone();
        let mut transform_bytes = Vec::new();

        for transform in &mut *transforms {
            transform.to_updated(); // update all at once for now...

            let matrix = transform.world_matrix();
            transform_bytes.extend_from_slice(&bytemuck::bytes_of(&matrix));
        }

        let transform_size = std::mem::size_of::<glam::Mat4>();
        let buffer_capacity = transforms.capacity() * transform_size;
        let transform_builder = BufferBuilder::as_vertex()
            .with_label(&transform_id.key)
            .with_capacity(buffer_capacity)
            .with_data(transform_bytes.clone());

        self.request_buffer(&transform_id, transform_builder);
        self.update_cmds.push(UpdateCommand { key: transform_id.clone(), data: transform_bytes });
    }

    /// Process an entity's material uniforms
    fn process_uniforms(&mut self, entity_desc: &EntityDesciptor, material: &Material) {
        fn resource_key(geometry_key: &String, resource_key: &String) -> String {
            format!("{}::{}", geometry_key, resource_key)
        }

        let mut bindings: Vec<ResourceBinding> = Vec::new();
        for (mut binding, uniform_builder) in material.get_uniforms() {
            match binding.id.scope {
                ResourceScope::Entity => {
                    binding.id.key = resource_key(&entity_desc.geometry_key, &binding.id.key);
                }
                _ => {}
            };

            match uniform_builder {
                UniformBuilder::Buffer(builder) => {
                    self.request_buffer(&binding.id, builder);
                }
                UniformBuilder::Texture(builder) => {
                    self.request_texture(&binding.id, builder);
                }
                UniformBuilder::Sampler(builder) => {
                    self.request_sampler(&binding.id, builder);
                }
            }

            bindings.push(binding);
        }

        // request bind group
        self.request_bind_group( &entity_desc.namespace, material.get_layout_builder(), bindings);

        for (mut uniform_id, data) in material.get_buffers_updated() {
            uniform_id.key = resource_key(&entity_desc.geometry_key, &uniform_id.key);

            self.update_cmds.push(UpdateCommand { key: uniform_id, data });
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

    /// request a create geometry command to be queued. Commands with the same key already queued will be skipped.
    fn request_geometry(&mut self, shader_path: &String, geometry_key: &String, builder: Arc<GeometryBuilder>) {
        self.create_cmds.push(CreateCommand::Geometry {
            shader_path: shader_path.clone(),
            key: geometry_key.clone(), 
            builder: Arc::clone(&builder),
        });
    }

    /// request a create shader spec command to be queued. Commands with the same key already queued will be skipped.
    fn request_shader_spec(&mut self, shader_path: &String) {
        if !self.tracker.as_mut().unwrap().shader_specs.contains(shader_path) {
            let builder = ShaderSpecPreset::from_known_path(&shader_path).unwrap();
            self.create_cmds.push(CreateCommand::ShaderSpec { 
                id: shader_path.clone(), 
                builder 
            });
        }
    }

    /// request a create render pipeline command to be queued. Commands with the same key already queued will be skipped.
    fn request_render_pipeline(&mut self, render_info: &RenderInfo) {
        if !self.tracker.as_mut().unwrap().pipelines.contains(&render_info.pipeline) {
            self.create_cmds.push(CreateCommand::RenderPipeline {
                shader_path: render_info.shader_path.clone(),
                builder: render_info.pipeline.clone(), 
                mode: InitMode::Deferred
            });
        }
    }

    /// request a create bind group command to be queued. Commands with the same key already queued will be skipped.
    fn request_bind_group(&mut self, id: &String, layout_id: BindGroupLayoutBuilder, bindings: Vec<ResourceBinding>) {
        if !self.tracker.as_mut().unwrap().bind_groups.contains(id) {
            self.create_cmds.push(CreateCommand::BindGroup {
                id: id.clone(),
                layout_id,
                bindings,
            });
        }
    }

    /// Take ownership of the renderer's tracker. Should only be called after all commands are recorded.
    pub(crate) fn take_tracker(&mut self) -> ResourceTracker {
        self.tracker.take().expect("Tracker already taken!")
    }
}
