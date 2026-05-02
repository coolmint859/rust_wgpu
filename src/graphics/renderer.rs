#![allow(dead_code)]
use std::sync::Arc;

use winit::window::Window;

use crate::graphics::{bind_group::BindGroupLayoutBuilder, entity::{Entity, EntityInstances}, geometry::GeometryID, init_state::StateInit, render_pipeline::RenderPipelineBuilder, wpgu_context::{ResourceBinding, ResourceID, WgpuContext}};

use super::{
    buffer::BufferBuilder, 
    camera::Camera, 
    init_state::InitMode, 
    material::UniformBuilder,
};

pub struct RenderContext {
    pub draw_cmds: Vec<DrawCommand>,
    pub instance_cmds: Vec<InstanceCommand>,
    pub bg_color: wgpu::Color,
    pub camera_key: String,
}

/// Command used to draw a single instance of a mesh to the current texture
#[derive(Clone, Debug)]
pub struct DrawCommand {
    /// used to get the geometry buffers
    pub geometry_id: GeometryID,
    /// used to get the entity's bind group
    pub instance_key: String,
    /// used to get the material's bind group
    pub material_key: String,
    /// used to get the render pipeline and shader
    pub pipeline_id: RenderPipelineBuilder,
    /// used for z-ordering
    pub z_depth: f32,
}

/// Command used to draw multiple instances of a mesh to the current texture
#[derive(Clone, Debug)]
pub struct InstanceCommand {
    /// used to get the geometry buffers
    pub geometry_id: GeometryID,
    /// used to get the transform buffer
    pub instance_id: ResourceID,
    /// used to get the material's bind group
    pub material_key: String,
    /// used to get the render pipeline
    pub pipeline_id: RenderPipelineBuilder,
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
    context: WgpuContext,

    // commands
    draw_cmds: Vec<DrawCommand>,
    instance_cmds: Vec<InstanceCommand>,

    // general frame settings
    clear_color: wgpu::Color,
    global_key: String,
    elapsed_time: f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        Self {
            context: WgpuContext::new(window).await,
            draw_cmds: Vec::new(),
            instance_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK,
            global_key: "".to_string(),
            elapsed_time: 0.0,
        }
    }

    pub fn init_resources(&mut self, init_state: StateInit) {
        self.context.init_resources(init_state);
    }

    /// resize the surface that the renderer renders to (also resizes the window)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.context.resize(width, height);
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

        self.context.process_buffer(&camera_id, &builder);
        self.context.update_resource(camera_id.clone(), &BufferBuilder::to_padded_vec(globals));
        
        self.context.process_bind_group(&camera_id.key, &camera.get_layout_builder(), vec![camera.get_binding()]);
        self.global_key = camera_id.key;
    }

    /// Draw an entity to the current texture
    pub fn draw(&mut self, entity: &mut Entity) {
        let geometry_id = match self.sync_entity(entity) {
            Some(id) => id,
            None => { return; }
        };

        self.draw_cmds.push(
            DrawCommand {
                geometry_id,
                instance_key: entity.transform_id().key,
                material_key: entity.material_key(),
                pipeline_id: entity.render_info.pipeline.clone(),
                z_depth: entity.transform.get_position().z,
            }
        );
    }

    /// Draw many instances of an entity to the current texture
    pub fn draw_instances(&mut self, instances: &mut EntityInstances) {
        let geometry_id = match self.sync_entity_instances(instances) {
            Some(id) => id,
            None => { return; }
        };

        self.instance_cmds.push(
            InstanceCommand {
                geometry_id,
                instance_id: instances.transform_id(),
                material_key: instances.material_key(),
                pipeline_id: instances.render_info.pipeline.clone(),
                instances: instances.transforms.data.len() as u32
            }
        );
    }

    /// Sync this entity's data with the wgpu context
    fn sync_entity(&mut self, mut entity: &mut Entity) -> Option<GeometryID>{
        let shader_path = entity.render_info.shader_path.clone();
        let spec = self.context.get_shader_spec(&shader_path)?;

        let geometry_id = GeometryID {
            key: entity.geometry.get_label(),
            attrs: spec.get_vertex_attributes()
        };

        self.context.process_geometry(&geometry_id, &entity.geometry);
        self.process_transform(&mut entity);
        self.process_uniforms(
            entity.get_uniforms(),
            entity.get_updated(),
            entity.material_key(),
            entity.material_layout()
        );
        self.context.process_pipeline(&entity.render_info, InitMode::Deferred);

        return Some(geometry_id);
    }

    fn sync_entity_instances(&mut self, mut instances: &mut EntityInstances) -> Option<GeometryID> {
        if instances.transforms.data.is_empty() { return None; }
        
        let shader_path = instances.render_info.shader_path.clone();
        let spec = self.context.get_shader_spec(&shader_path)?;

        let geometry_id = GeometryID {
            key: instances.geometry.get_label(),
            attrs: spec.get_vertex_attributes()
        };

        self.context.process_geometry(&geometry_id, &instances.geometry);
        self.process_instance_transforms(&mut instances);
        self.process_uniforms(
            instances.get_uniforms(),
            instances.get_updated(),
            instances.material_key(),
            instances.material_layout()
        );
        self.context.process_pipeline(&instances.render_info, InitMode::Deferred);

        return Some(geometry_id);
    }

    /// Request an entity's transform layout and buffer 
    fn process_transform(&mut self, entity: &mut Entity) {
        let transform_id = entity.transform_id();
        self.context.process_bind_group(&transform_id.key, &entity.transform_layout(), vec![entity.transform_binding()]);

        // update buffers if transform had changed (is guarenteed to on first frame)
        if entity.transform.to_updated() {
            let matrix_data = bytemuck::bytes_of(&entity.transform.world_matrix()).to_vec();

            let uniform_builder = BufferBuilder::as_uniform()
                .with_label(&transform_id.key)
                .with_data(matrix_data.clone());

            self.context.process_buffer(&transform_id, &uniform_builder);
            self.context.update_resource(transform_id.clone(), &matrix_data);
        }
    }

    /// Process transforms from multiple instances
    fn process_instance_transforms(&mut self, instances: &mut EntityInstances) {
        let transform_id = instances.transform_id();
        let mut transform_bytes = Vec::new();

        for transform in instances.transforms.data.iter_mut() {
            transform.to_updated(); // update all at once for now...

            let matrix = transform.world_matrix();
            transform_bytes.extend_from_slice(&bytemuck::bytes_of(&matrix));
        }

        let transform_size = std::mem::size_of::<glam::Mat4>();
        let buffer_capacity = instances.transforms.data.capacity() * transform_size;
        let transform_builder = BufferBuilder::as_vertex()
            .with_label(&transform_id.key)
            .with_capacity(buffer_capacity)
            .with_data(transform_bytes.clone());

        self.context.process_buffer(&transform_id, &transform_builder);
        self.context.update_resource(transform_id.clone(), &transform_bytes);
    }

    /// Process an entity's material uniforms
    fn process_uniforms(&mut self,
        uniforms: Vec<(ResourceBinding, UniformBuilder)>,
        updated: Vec<(ResourceID, Vec<u8>)>,
        material_key: String, 
        material_layout: BindGroupLayoutBuilder 
    ) {
        let mut bindings = Vec::new();

        for (binding, uniform_builder) in uniforms {
            match uniform_builder {
                UniformBuilder::Buffer(builder) => self.context.process_buffer(&binding.id, &builder),
                UniformBuilder::Texture(builder) => self.context.process_texture(&binding.id, &builder),
                UniformBuilder::Sampler(builder) => self.context.process_sampler(&binding.id, &builder),
            }
            bindings.push(binding);
        }

        self.context.process_bind_group(&material_key, &material_layout, bindings);

        for (uniform_id, data) in updated {
            self.context.update_resource(uniform_id, &data);
        }
    }

    pub fn begin_frame(&mut self, elapsed_time: f32) {
        self.draw_cmds.clear();
        self.instance_cmds.clear();

        self.context.prepare_next_frame();
        self.elapsed_time = elapsed_time;
    }

    pub fn end_frame(&mut self) {
        // prevents copying commands over
        let mut draw_cmds = Vec::new();
        std::mem::swap(&mut self.draw_cmds, &mut draw_cmds);

        let mut instance_cmds = Vec::new();
        std::mem::swap(&mut self.instance_cmds, &mut instance_cmds);

        let render_ctx = RenderContext {
            draw_cmds,
            instance_cmds,
            bg_color: self.clear_color.clone(),
            camera_key: self.global_key.clone()
        };

        let _ = self.context.render(render_ctx);
    }
}
