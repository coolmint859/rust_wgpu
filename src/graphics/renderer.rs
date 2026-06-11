#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use glam::{Mat4, Quat, Vec3, Vec4};
use winit::window::Window;

use crate::graphics::{font::{CharacterGlyph, Font}, geometry::Geometry, init_state::StateInit, instance::{InstanceGroup, InstanceTemplate}, primitive::Primitive, render_pipeline::RenderPipelineBuilder, transform::Transform, vertex::attr, wpgu_context::{GeometryID, ResourceID, ResourceUpdate, WgpuContext}};

use super::{
    buffer::BufferBuilder, 
    camera::Camera, 
    init_state::InitMode, 
    material::UniformBuilder,
};

/// Options for text display
pub struct TextOptions {
    /// the position of the text (top left corner)
    pub pos: Vec3,
    /// the color of the text
    pub text_color: Vec4,
    /// the color of the outline. If None, then the text is rendered with no outline
    pub outline_color: Option<Vec4>,
    /// the width of the longest line in a text string (NDC space)
    pub width: f32,
}

/// Used to pass frame by frame rendering commands to the WGPU Context
pub struct RenderContext {
    /// The set of pending draw commands
    pub draw_cmds: Vec<DrawCommand>,
    /// the render surface clear color
    pub bg_color: wgpu::Color,
    /// the id into the global bind group
    pub global_id: ResourceID,
}

/// Command used to draw primitives to the current texture
#[derive(Clone, Debug)]
pub struct DrawCommand {
    /// used to get the geometry buffers
    pub geometry_id: GeometryID,
    /// used to get the primitive's transform buffer
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

/// Constructs render commands from primitives.
/// 
/// This acts as a translator for high level constructs into low level data 
/// for the WgpuContext during a single frame.
pub struct Renderer {
    /// the wpgu rendering context
    context: WgpuContext,
    /// the set of draw commands per frame
    draw_cmds: Vec<DrawCommand>,
    /// the map of resource ids to font primitives used for rendering text
    font_primitives: HashMap<ResourceID, Primitive>,

    /// the background clear color
    clear_color: wgpu::Color,
    /// the camera view matrix
    camera_view: Mat4,
    /// the bind group id for global uniforms
    global_id: Option<ResourceID>,
    /// the elapsed time since the start of the program
    elapsed_time: f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        Self {
            context: WgpuContext::new(window).await,
            draw_cmds: Vec::new(),
            font_primitives: HashMap::new(),
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

    /// Draw a primitive to the current texture
    pub fn draw(&mut self, primitive: &mut Primitive) {
        if primitive.instances.count() == 0 { return; }

        self.context.process_shader_spec(&primitive.render_info.shader_path);
        self.process_geometry(primitive.geometry_id(), &primitive.geometry);
        self.process_instances(primitive.instance_id(), &mut primitive.instances);
        self.process_material(primitive);
        self.context.process_pipeline(&primitive.render_info, InitMode::Deferred);

        let position = primitive.first().transform().get_position();
        let view_pos = self.camera_view * Vec4::new(position.x, position.y, position.z, 1.0);
        let z_depth = view_pos.z;

        self.draw_cmds.push(
            DrawCommand {
                geometry_id: primitive.geometry.get_ids(),
                instance_id: primitive.instance_id(),
                material_id: primitive.material_id(),
                pipeline_id: primitive.render_info.pipeline.clone(),
                instances: primitive.instances.count() as u32,
                z_depth,
            }
        );
    }

    /// Draw text to the current texture
    /// 
    /// * 'text' - the string of text to render
    /// * 'font'- the font to render the text with
    /// * 'options' the text display options
    pub fn draw_text(&mut self, text: &str, font: &Font, options: TextOptions) {
        let font_id = font.get_id();
        self.context.process_font(&font_id, &font.get_builder());
        
        if let Some(font_asset) = self.context.get_font(&font_id) {
            if !self.font_primitives.contains_key(&font_id) {
                let font_prim = font.create_primitive(1000);
                self.font_primitives.insert(font_id.clone(), font_prim);
            }

            let font_prim = self.font_primitives.get_mut(&font_id).unwrap();

            let size = font_asset.calc_size(text, options.width);
            let template = font_prim.get_template().with_defaults();

            let mut cursor = options.pos.clone();
            for character in text.chars() {
                if character == '\n' {
                    cursor.x = options.pos.x;
                    cursor.y -= font_asset.line_height * size;

                    continue;
                }

                if let Some(glyph) = font_asset.glyphs.get(&character) {
                    // prevent spaces from taking up an instance.
                    if character == ' ' {
                        cursor.x += glyph.advance * size;
                        continue; 
                    }

                    let outline_color = options.outline_color.unwrap_or(Vec4::ZERO);
                    let instance = Renderer::create_char_instance(template.clone(), glyph, &cursor, size, options.text_color, outline_color);
                    font_prim.instances.add_instance(instance);

                    cursor.x += glyph.advance * size;
                }
            }
        }
    }

    /// create a new character instance for the font primitive from the provided glyph
    pub fn create_char_instance(
        mut template: InstanceTemplate, 
        glyph: &CharacterGlyph, 
        cursor: &Vec3, 
        size: f32, 
        text_color: Vec4,
        outline_color: Vec4
    ) -> InstanceTemplate {
        let x_scale = glyph.plane_bounds.z * size;
        let y_scale = glyph.plane_bounds.w * size;
        
        let x_pos = cursor.x + (glyph.plane_bounds.x * size) + (x_scale * 0.5);
        let y_pos = cursor.y + (glyph.plane_bounds.y * size) + (y_scale * 0.5);

        let transform = Transform::new(
            Vec3::new(x_pos, y_pos, cursor.z),
            Quat::IDENTITY,
            Vec3::new(x_scale, y_scale, 1.0)
        );

        template.set_attribute(attr::TRANSFORM, transform);
        template.set_attribute(attr::UV_BOUNDS, glyph.uv_bounds);
        template.set_attribute(attr::TEXT_COLOR, text_color);
        template.set_attribute(attr::OUTLINE_COLOR, outline_color);

        template
    }

    /// Process the geometry of an entity
    fn process_geometry(&mut self, geometry_id: GeometryID, geometry: &Geometry) {
        if !self.context.contains_buffer(&geometry_id.vertex_id) {
            self.context.process_geometry(geometry.get_signature());
        }
    }

    /// Process the instance group of an entity
    fn process_instances(&mut self, instance_id: ResourceID, instances: &mut InstanceGroup) {
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
    fn process_material(&mut self, entity: &mut Primitive) {
        let mut bindings = Vec::new();

        for (binding, uniform_builder) in entity.get_uniforms() {
            match uniform_builder {
                UniformBuilder::Buffer(builder) => self.context.process_buffer(&binding.id, &builder),
                UniformBuilder::Texture(builder) => self.context.process_texture(&binding.id, &builder),
                UniformBuilder::Sampler(builder) => self.context.process_sampler(&binding.id, &builder),
                _ => {}
            }
            bindings.push(binding);
        }

        self.context.process_bind_group(&entity.material_id(), &entity.material.get_layout_builder(), bindings);

        for (id, update) in entity.uniform_updates() {
            self.context.update_resource(&id, update);
        }
    }

    /// begin a new frame to prepare for rendering. 
    pub fn begin_frame(&mut self, elapsed_time: f32) {
        self.context.prepare_next_frame();
        self.elapsed_time = elapsed_time;

        // loop through every font primitive and clear the character buffers
        for prim in self.font_primitives.values_mut() {
            prim.instances.clear_instances();
        }
    }

    /// End the current frame and finalize draw commands
    pub fn end_frame(&mut self) {
        // submit draw calls for known fonts
        let mut font_primitives = std::mem::take(&mut self.font_primitives);
        for font_prim in font_primitives.values_mut() {
            self.draw(font_prim);
        }
        self.font_primitives = font_primitives;

        // prevents copying commands over
        let mut draw_cmds = Vec::new();
        std::mem::swap(&mut self.draw_cmds, &mut draw_cmds);

        // sort the commands based on depth value back to front
        draw_cmds.sort_by(|a, b| {
            b.z_depth.partial_cmp(&a.z_depth).unwrap_or(std::cmp::Ordering::Equal)
        });

        // pass draw commands and rendering options to context for gpu execution
        let render_ctx = RenderContext {
            draw_cmds,
            bg_color: self.clear_color.clone(),
            global_id: self.global_id.clone().unwrap() 
        };

        let _ = self.context.render(render_ctx);
    }
}
