#![allow(dead_code)]
use std::sync::Arc;

use winit::window::Window;

use crate::graphics::{entity::Entity, geometry::GeometryID, init_state::StateInit, render_pipeline::RenderPipelineBuilder, transform::Transform, wpgu_context::{ResourceID, ResourceUpdate, WgpuContext}};

use super::{
    buffer::BufferBuilder, 
    camera::Camera, 
    init_state::InitMode, 
    material::UniformBuilder,
};

pub struct RenderContext {
    pub draw_cmds: Vec<DrawCommand>,
    pub bg_color: wgpu::Color,
    pub camera_key: String,
}

/// Command used to draw a single instance of a mesh to the current texture
#[derive(Clone, Debug)]
pub struct DrawCommand {
    /// used to get the geometry buffers
    pub geometry_id: GeometryID,
    /// used to get the entity's transform buffer
    pub instance_id: ResourceID,
    /// used to get the material's bind group
    pub material_key: String,
    /// used to get the render pipeline and shader
    pub pipeline_id: RenderPipelineBuilder,
    /// the number of instances to draw
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

    draw_cmds: Vec<DrawCommand>,
    clear_color: wgpu::Color,
    global_key: String,
    elapsed_time: f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        Self {
            context: WgpuContext::new(window).await,
            draw_cmds: Vec::new(),
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
        let camera_id = camera.get_id();
        let globals = GlobalUniforms {
            view_proj: camera.get_view_proj_mat().to_cols_array(),
            cam_pos: camera.get_position().to_array(),
            elapsed_time: self.elapsed_time,
        };

        if !self.context.contains_buffer(&camera_id) {
            let builder = BufferBuilder::as_uniform()
                .with_label(&camera_id.key)
                .with_data_from_struct(globals);
            self.context.process_buffer(&camera_id, &builder);
        } else {
            camera.to_updated();
            let update = ResourceUpdate {
                id: camera_id.clone(),
                offset: 0,
                data: BufferBuilder::to_padded_vec(globals)
            };
            self.context.update_resource(update);
        }

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
                instance_id: entity.transform_id(),
                material_key: entity.material_key(),
                pipeline_id: entity.render_info.pipeline.clone(),
                instances: entity.transforms.len() as u32
            }
        );
    }

    /// Sync this entity's data with the wgpu context
    fn sync_entity(&mut self, mut entity: &mut Entity) -> Option<GeometryID>{
        let shader_path = entity.render_info.shader_path.clone();
        let spec = self.context.init_shader_spec(&shader_path)?;

        let geometry_id = GeometryID {
            key: entity.geometry.get_label(),
            attrs: spec.get_vertex_attributes()
        };

        self.context.process_geometry(&geometry_id, &entity.geometry);

        self.process_transforms(&mut entity);
        self.process_uniforms(&mut entity);
        self.context.process_pipeline(&entity.render_info, InitMode::Deferred);

        return Some(geometry_id);
    }

    /// Process transforms from multiple instances
    fn process_transforms(&mut self, entity: &mut Entity) {
        let transform_id = entity.transform_id();

        if !self.context.contains_buffer(&transform_id) {
            let init_update = entity.transform_updates(true);
            let init_data = init_update[0].data.clone();

            let capacity = entity.transforms.capacity() * Transform::size();
            let transform_builder = BufferBuilder::as_vertex()
                .with_label(&transform_id.key)
                .with_capacity(capacity)
                .with_data(init_data);

            self.context.process_buffer(&transform_id, &transform_builder);
        } else {
            for update in entity.transform_updates(false) {
                self.context.update_resource(update);
            }
        }
    }

    /// Process an entity's material uniforms
    fn process_uniforms(&mut self, entity: &mut Entity) {
        let mut bindings = Vec::new();

        for (binding, uniform_builder) in entity.get_uniforms() {
            match uniform_builder {
                UniformBuilder::Buffer(builder) => self.context.process_buffer(&binding.id, &builder),
                UniformBuilder::Texture(builder) => self.context.process_texture(&binding.id, &builder),
                UniformBuilder::Sampler(builder) => self.context.process_sampler(&binding.id, &builder),
            }
            bindings.push(binding);
        }

        self.context.process_bind_group(&entity.material_key(), &entity.material_layout(), bindings);

        for update in entity.uniform_updates() {
            self.context.update_resource(update);
        }
    }

    pub fn begin_frame(&mut self, elapsed_time: f32) {
        self.context.prepare_next_frame();
        self.elapsed_time = elapsed_time;
    }

    pub fn end_frame(&mut self) {
        // prevents copying commands over
        let mut draw_cmds = Vec::new();
        std::mem::swap(&mut self.draw_cmds, &mut draw_cmds);

        let render_ctx = RenderContext {
            draw_cmds,
            bg_color: self.clear_color.clone(),
            camera_key: self.global_key.clone()
        };

        let _ = self.context.render(render_ctx);
    }
}
