#![allow(dead_code)]
use std::sync::Arc;

use glam::{Mat4, Vec4};
use winit::window::Window;

use crate::graphics::{entity::Entity, geometry::Geometry, init_state::StateInit, instance::InstanceGroup, render_pipeline::RenderPipelineBuilder, wpgu_context::{GeometryID, ResourceID, ResourceUpdate, WgpuContext}};

use super::{
    buffer::BufferBuilder, 
    camera::Camera, 
    init_state::InitMode, 
    material::UniformBuilder,
};

/// 
pub struct RenderContext {
    /// The set of pending draw commands
    pub draw_cmds: Vec<DrawCommand>,
    /// the render surface clear color
    pub bg_color: wgpu::Color,
    /// the id into the global bind group
    pub global_id: ResourceID,
}

/// Command used to draw instances of an entity to the current texture
#[derive(Clone, Debug)]
pub struct DrawCommand {
    /// used to get the geometry buffers
    pub geometry_id: GeometryID,
    /// used to get the entity's transform buffer
    pub instance_id: ResourceID,
    /// used to get the material's bind group
    pub material_id: ResourceID,
    /// used to get the render pipeline and shader
    pub pipeline_id: RenderPipelineBuilder,
    /// the number of instances to draw
    pub instances: u32,
    /// The z value of the first instances's position
    pub z_depth: f32,
}

/// Uniforms that are global to the entire scene
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlobalUniforms {
    /// Camera view-projection matrix
    view_proj: [f32; 16],
    /// Camera world space position
    cam_pos: [f32; 3],
    /// Time since the start of program
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
    camera_view: Mat4,
    global_id: Option<ResourceID>,
    elapsed_time: f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        Self {
            context: WgpuContext::new(window).await,
            draw_cmds: Vec::new(),
            clear_color: wgpu::Color::BLACK,
            global_id: None,
            camera_view: Mat4::IDENTITY,
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
                offset: 0,
                data: BufferBuilder::to_padded_vec(globals)
            };
            self.context.update_resource(&camera_id, update);
        }

        self.context.process_bind_group(&camera_id, &camera.get_layout_builder(), vec![camera.get_binding()]);
        
        self.camera_view = camera.view_matrix();
        self.global_id = Some(camera_id);
    }

    /// Draw an entity to the current texture
    pub fn draw(&mut self, entity: &mut Entity) {
        if entity.instances.count() == 0 { return; }

        self.context.process_shader_spec(&entity.render_info.shader_path);
        self.process_geometry(entity.geometry_id(), &entity.geometry);
        self.process_instances(entity.instance_id(), &entity.instances);
        self.process_material(entity);
        self.context.process_pipeline(&entity.render_info, InitMode::Deferred);

        let entity_pos = entity.first().transform().get_position();
        let view_pos = self.camera_view * Vec4::new(entity_pos.x, entity_pos.y, entity_pos.z, 1.0);
        let z_depth = view_pos.z;

        self.draw_cmds.push(
            DrawCommand {
                geometry_id: entity.geometry.get_ids(),
                instance_id: entity.instance_id(),
                material_id: entity.material_id(),
                pipeline_id: entity.render_info.pipeline.clone(),
                instances: entity.instances.count() as u32,
                z_depth,
            }
        );
    }

    /// Process the geometry of an entity
    fn process_geometry(&mut self, geometry_id: GeometryID, geometry: &Geometry) {
        if !self.context.contains_buffer(&geometry_id.vertex_id) {
            self.context.process_geometry(geometry.get_signature());
        }
    }

    /// Process the instance group of an entity
    fn process_instances(&mut self, instance_id: ResourceID, instances: &InstanceGroup) {
        if !self.context.contains_buffer(&instance_id) {
            self.context.process_buffer(&instance_id, &instances.get_buffer_builder());
        } else {
            let updates = instances.get_updated();
            for update in updates {
                self.context.update_resource(&instance_id, update);
            }
        }
    }

    /// Process the material of an entity
    fn process_material(&mut self, entity: &mut Entity) {
        let mut bindings = Vec::new();

        for (binding, uniform_builder) in entity.get_uniforms() {
            match uniform_builder {
                UniformBuilder::Buffer(builder) => self.context.process_buffer(&binding.id, &builder),
                UniformBuilder::Texture(builder) => self.context.process_texture(&binding.id, &builder),
                UniformBuilder::Sampler(builder) => self.context.process_sampler(&binding.id, &builder),
            }
            bindings.push(binding);
        }

        self.context.process_bind_group(&entity.material_id(), &entity.material.get_layout_builder(), bindings);

        for (id, update) in entity.uniform_updates() {
            self.context.update_resource(&id, update);
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

        // sort the commands back to front
        draw_cmds.sort_by(|a, b| {
            b.z_depth.partial_cmp(&a.z_depth).unwrap_or(std::cmp::Ordering::Equal)
        });

        let render_ctx = RenderContext {
            draw_cmds,
            bg_color: self.clear_color.clone(),
            global_id: self.global_id.clone().unwrap() 
        };

        let _ = self.context.render(render_ctx);
    }
}
