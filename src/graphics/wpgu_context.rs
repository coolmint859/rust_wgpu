#![allow(dead_code)]
use wgpu::TextureView;
use winit::window::Window;
use std::sync::Arc;

use crate::graphics::{
    bind_group::{BindGroupBuilder, BindGroupContext, BindGroupLayoutBuilder, BindGroupResource}, buffer::{BufferBuilder, BufferContext}, core::WgpuCore, entity::RenderInfo, geometry::{GeometryBuffer, GeometryBuilder, GeometryContext, GeometryID}, handler::{ResourceHandler, ResourceStatus}, init_state::{InitMode, StateInit}, presets::{ShaderSpecPreset, TextureSampler}, render_pipeline::{RenderPipelineBuilder, RenderPipelineContext}, renderer::{DrawCommand, InstanceCommand, RenderContext}, shader::{ShaderSpec, ShaderSpecBuilder}, texture::{SamplerBuilder, TextureBuilder, TextureContext},
};

/// Group binding number for global uniforms
pub const GLOBAL_UNIFORMS: u32 = 0;
/// Group binding number for material uniforms
pub const MATERIAL_UNIFORMS: u32 = 1;
/// Group binding number for instance uniforms
pub const INSTANCE_UNIFORMS: u32 = 2;

/// Specifies the scope for which a resource should be namespaced (allows different levels of resource sharing)
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub enum ResourceScope {
    /// Resources are scoped to individual entities; no sharing (best used for buffers)
    Entity,
    /// Resources are scoped to materials; shared between entities (best used for textures)
    Material,
    /// Resources are globally scoped; shared everywhere (best used for samplers)
    Global
}

/// The idenitifer of a gpu resource, including its unique key and scope
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ResourceID {
    /// The unqiue key for a resource
    pub key: String,
    /// The namespaced scope of a resource
    pub scope: ResourceScope,
}

/// The specific binding of a resource when used in a bind group
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ResourceBinding {
    /// the id of the resource
    pub id: ResourceID,
    /// the bind slot the resource will be placed in a matching shader
    pub slot: u32,
}

/// Represents the entire WebGPU rendering context. 
/// 
/// Coordinates syncronization of gpu resources created on handler worker threads with the main thread.
/// 
/// Accepts commands generated to generate and update resources, and render objects.
pub struct WgpuContext {
    core: WgpuCore,

    layout_handler: ResourceHandler<BindGroupLayoutBuilder, Arc<wgpu::BindGroupLayout>>,
    pipeline_handler: ResourceHandler<RenderPipelineBuilder, wgpu::RenderPipeline>,
    shader_spec_handler: ResourceHandler<String, ShaderSpec>,
    shader_mod_handler: ResourceHandler<String, Arc<wgpu::ShaderModule>>,

    buffer_handler: ResourceHandler<ResourceID, Arc<wgpu::Buffer>>,
    texture_handler: ResourceHandler<ResourceID, Arc<TextureView>>,
    sampler_handler: ResourceHandler<ResourceID, Arc<wgpu::Sampler>>,

    geometry_handler: ResourceHandler<GeometryID, GeometryBuffer>,
    bindgroup_handler: ResourceHandler<String, wgpu::BindGroup>,
}

impl WgpuContext {
    pub async fn new(window: Arc<Window>) -> Self {
        let core = WgpuCore::new(window).await;

        let sampler_handler = WgpuContext::init_samplers(&core);
        let shader_spec_handler = WgpuContext::init_shader_specs();
        
        Self {
            core,
            sampler_handler,
            shader_spec_handler,
            layout_handler: ResourceHandler::new(),
            pipeline_handler: ResourceHandler::new(),
            shader_mod_handler: ResourceHandler::new(),
            buffer_handler: ResourceHandler::new(),
            texture_handler: ResourceHandler::new(),
            geometry_handler: ResourceHandler::new(),
            bindgroup_handler: ResourceHandler::new(),
        }
    }

    fn init_shader_specs() -> ResourceHandler<String, ShaderSpec> {
        let colored_sprite = ShaderSpecPreset::ColoredSprite;
        let colored_instanced = ShaderSpecPreset::ColoredSpriteInstanced;
        let textured_sprite = ShaderSpecPreset::TexturedSprite;
        let textured_instanced = ShaderSpecPreset::TexturedSpriteInstanced;

        let mut spec_handler = ResourceHandler::new();
        spec_handler.request_new(&colored_sprite.path(), &colored_sprite.get(), Arc::new(()));
        spec_handler.request_new(&colored_instanced.path(), &colored_instanced.get(), Arc::new(()));
        spec_handler.request_new(&textured_sprite.path(), &textured_sprite.get(), Arc::new(()));
        spec_handler.request_new(&textured_instanced.path(), &textured_instanced.get(), Arc::new(()));

        spec_handler
    }

    fn init_samplers(core: &WgpuCore) -> ResourceHandler<ResourceID, Arc<wgpu::Sampler>> {
        let mut sampler_handler = ResourceHandler::new();
        let _ = sampler_handler.request_wait(
            &ResourceID { 
                key: TextureSampler::NearestClampToEdge.label(),
                scope: ResourceScope::Global 
            }, 
            &TextureSampler::NearestClampToEdge.get(), 
            Arc::clone(&core.device)
        );
        let _ = sampler_handler.request_wait(
            &ResourceID { 
                key: TextureSampler::NearestRepeat.label(),
                scope: ResourceScope::Global 
            },
            &TextureSampler::NearestRepeat.get(), 
            Arc::clone(&core.device)
        );

        sampler_handler
    }

    /// initialize resources prior to rendering state
    pub fn init_resources(&mut self, init_state: StateInit) {
        for rpip_cmd in init_state.get_rpip_cmds() {
            let render_info = RenderInfo {
                shader_path: rpip_cmd.shader_path.clone(),
                pipeline: rpip_cmd.builder.clone()
            };
            self.process_pipeline(&render_info, InitMode::Deferred);
        }
        
        for bgl_cmd in init_state.get_bgl_cmds() {
            match bgl_cmd.mode {
                InitMode::Immediate => {
                    let _ = self.layout_handler.request_wait(&bgl_cmd.builder, &bgl_cmd.builder, Arc::clone(&self.core.device));
                },
                InitMode::Deferred => {
                    self.layout_handler.request_new(&bgl_cmd.builder, &bgl_cmd.builder, Arc::clone(&self.core.device));
                }
            }
        }
    }

    /// initialize a shader specification and associated bind group layouts
    pub fn process_shader_spec(&mut self, id: &String, builder: ShaderSpecBuilder) {
        if let Some(spec) = self.shader_spec_handler.get(&id) {
            for layout in &spec.bg_layouts {
                let device_cpy = self.core.device.clone();
                self.layout_handler.request_new(&layout, layout, device_cpy);
            }
        } else {
            self.shader_spec_handler.request_new(&id, &builder, Arc::new(()));
        }
    }

    pub fn get_shader_spec(&mut self, path: &String) -> Option<ShaderSpec> {
        if let Some(spec) = self.shader_spec_handler.get(path) {
            for layout in &spec.bg_layouts {
                if self.layout_handler.is_ready(layout) { continue; }

                let device_cpy = self.core.device.clone();
                self.layout_handler.request_new(&layout, layout, device_cpy);
            }

            return Some(spec.clone());
        } else {
            let builder = ShaderSpecPreset::from_known_path(path).unwrap();
            self.shader_spec_handler.request_new(path, &builder, Arc::new(()));
            return None;
        }
    }

    /// Initialize a new pipeline request
    pub fn process_pipeline(&mut self, render_info: &RenderInfo, mode: InitMode) {
        let shader_spec = match self.shader_spec_handler.get(&render_info.shader_path) {
            Some(spec) => spec,
            None => { return; }
        };

        let shader_mod = match self.shader_mod_handler.get(&render_info.shader_path) {
            Some(module ) => module,
            None => { 
                let device_cpy = self.core.device.clone();
                self.shader_mod_handler.request_new(&render_info.shader_path, &shader_spec.shader_builder, device_cpy);
                return; 
            }
        };

        let mut layouts = Vec::new();
        for id in &shader_spec.bg_layouts {
            if let Some(layout) = self.layout_handler.get(&id) {
                layouts.push(Arc::clone(layout));
            }
        }

        if layouts.len() == shader_spec.bg_layouts.len() {
            let rpip_context = Arc::new(
                RenderPipelineContext {
                    device: Arc::clone(&self.core.device),
                    bg_layouts: layouts,
                    format: self.core.config.format.clone(),
                    shader: shader_mod.clone(),
                    shader_spec: shader_spec.clone()
                }
            );

            match mode {
                InitMode::Immediate => {
                    let _ = self.pipeline_handler.request_wait(
                        &render_info.pipeline, 
                        &render_info.pipeline, 
                        Arc::clone(&rpip_context)
                    );
                },
                InitMode::Deferred => {
                    self.pipeline_handler.request_new(
                        &render_info.pipeline, 
                        &render_info.pipeline, 
                        Arc::clone(&rpip_context)
                    );
                }
            }
        }
    }

    /// initialize a new bind group request 
    pub fn process_bind_group(&mut self, group_id: &String, layout_id: &BindGroupLayoutBuilder, bindings: Vec<ResourceBinding>) {
        let layout = match self.layout_handler.get(layout_id) {
            Some(layout) => layout,
            None => { return; }
        };

        let mut resource_pairs = Vec::with_capacity(bindings.len());
        for binding in &bindings {
            // check for buffer
            if let Some(buffer) = self.buffer_handler.get(&binding.id) {
                resource_pairs.push((binding.slot, BindGroupResource::Buffer(Arc::clone(buffer))))
            }
            // check for texture
            if let Some(texture_view) = self.texture_handler.get(&binding.id) {
                resource_pairs.push((binding.slot, BindGroupResource::Texture(Arc::clone(&texture_view))));
            }
            // check for sampler
            if let Some(sampler) = self.sampler_handler.get(&binding.id) {
                resource_pairs.push((binding.slot, BindGroupResource::Sampler(Arc::clone(&sampler))));
            }
        }

        // println!("found resources: {:?}, expected respources: {:?}", resource_pairs, bindings);

        // all resources found, safe to create bind group
        if resource_pairs.len() == bindings.len() {
            let builder = BindGroupBuilder::new()
                .with_label(&group_id)
                .with_resources(resource_pairs);

            let context = Arc::new(BindGroupContext {
                device: Arc::clone(&self.core.device),
                layout: Arc::clone(&layout)
            });

            self.bindgroup_handler.request_new(&group_id, &builder, context);
        }
    }

    /// initialize a new buffer request
    pub fn process_buffer(&mut self, id: &ResourceID, builder: &BufferBuilder) {
        let context = Arc::new(BufferContext {
            device: Arc::clone(&self.core.device),
            queue: Arc::clone(&self.core.queue)
        });

        self.buffer_handler.request_new(&id, builder, context);
    }

    /// initialize a new texture request
    pub fn process_texture(&mut self, key: &ResourceID, builder: &TextureBuilder) {
        let context = Arc::new(TextureContext {
            device: Arc::clone(&self.core.device),
            queue: Arc::clone(&self.core.queue),
        });

        self.texture_handler.request_new(&key, builder, context);
    }

    /// initialize a new sampler request
    pub fn process_sampler(&mut self, id: &ResourceID, builder: &SamplerBuilder) {
        self.sampler_handler.request_new(&id, builder, Arc::clone(&self.core.device));
    }

    pub fn process_geometry(&mut self, geometry_id: &GeometryID, builder: &GeometryBuilder) {
        let buffer_context = Arc::new(BufferContext {
            device: Arc::clone(&self.core.device),
            queue: Arc::clone(&self.core.queue)
        });

        let geometry_context = Arc::new(GeometryContext {
            buffer_context,
            attrs: geometry_id.attrs.clone(),
        });

        self.geometry_handler.request_new(&geometry_id, builder, geometry_context);
    }

    /// Prepare the context for the next frame
    pub fn prepare_next_frame(&mut self) {
        self.layout_handler.sync();
        self.geometry_handler.sync();
        self.pipeline_handler.sync();
        self.bindgroup_handler.sync();
        self.buffer_handler.sync();
        self.texture_handler.sync();
        self.sampler_handler.sync();
        self.shader_spec_handler.sync();
        self.shader_mod_handler.sync();

        self.core.window.request_redraw();
    }

    /// resize the surface that the context renders to (also resizes the window)
    pub fn resize(&mut self, width: u32, height: u32) {
        self.core.resize(width, height);
    }

    /// process update commands
    pub fn update_resource(&mut self, key: ResourceID, data: &Vec<u8>) {
        if let Some(buffer) = self.buffer_handler.get(&key) {
            self.core.queue.write_buffer(buffer, 0, &data);
        }
        // add texture check when ready
    }

    /// render commands given to the renderer instance
    pub fn render(&mut self, render_ctx: RenderContext) -> anyhow::Result<()> {
        if !self.core.is_surface_configured() {
            return Ok(());
        }

        // verify camera existence
        let camera_group = match self.bindgroup_handler.get(&render_ctx.camera_key) {
            Some(data) => data,
            None => return Ok(()) // if the camera bind group is not ready, we can't draw anything
        };

        // prepare output and render pass
        let output = self.core.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.core.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") }
        );

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(render_ctx.bg_color),
                            store: wgpu::StoreOp::Store,
                        }
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                }
            );

            // draw meshes to current texture
            render_pass.set_bind_group(GLOBAL_UNIFORMS, camera_group, &[]);
            self.draw_single(&render_ctx.draw_cmds, &mut render_pass);
            self.draw_instances(&render_ctx.instance_cmds, &mut render_pass);
        }

        self.core.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// draw meshes to the current texture using the provided render pass
    fn draw_single(&mut self, draw_cmds: &Vec<DrawCommand>, render_pass: &mut wgpu::RenderPass) {
        for draw_cmd in draw_cmds {
            let geo_status = self.geometry_handler.status_of(&draw_cmd.geometry_id);
            let pip_status = self.pipeline_handler.status_of(&draw_cmd.pipeline_id.clone());
            let u_mat_status = self.bindgroup_handler.status_of(&draw_cmd.material_key.clone());
            let u_instance_status = self.bindgroup_handler.status_of(&draw_cmd.instance_key.clone());

            // println!("mesh ready? {}", mesh_status.is_some());
            // println!("pipeline ready? {}", pip_status.is_some());
            // println!("material ready? {}", mat_u_status.is_some());
            // println!("transform ready? {}", mesh_u_status.is_some());

            if let (Some(ResourceStatus::Ready(geometry)), 
                    Some(ResourceStatus::Ready(pipeline)), 
                    Some(ResourceStatus::Ready(u_material)),
                    Some(ResourceStatus::Ready(u_instance))) = (geo_status, pip_status, u_mat_status, u_instance_status) 
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(MATERIAL_UNIFORMS, u_material, &[]);
                render_pass.set_bind_group(INSTANCE_UNIFORMS, u_instance, &[]);
                render_pass.set_vertex_buffer(0, geometry.vertex_buffer.slice(..));
                render_pass.set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..geometry.num_indices, 0, 0..1);
            }
        }
    }

    fn draw_instances(&mut self, instance_cmds: &Vec<InstanceCommand>, render_pass: &mut wgpu::RenderPass) {
        for insance_cmd in instance_cmds {
            let geo_status = self.geometry_handler.status_of(&insance_cmd.geometry_id);
            let pip_status = self.pipeline_handler.status_of(&insance_cmd.pipeline_id.clone());
            let u_mat_status = self.bindgroup_handler.status_of(&insance_cmd.material_key.clone());
            let u_instance_status = self.buffer_handler.status_of(&insance_cmd.instance_id);

            // println!("mesh ready? {}", mesh_status.is_some());
            // println!("pipeline ready? {}", pip_status.is_some());
            // println!("material ready? {}", mat_u_status.is_some());
            // println!("transforms ready? {}", transforms_status.is_some());

            if let (Some(ResourceStatus::Ready(geometry)), 
                    Some(ResourceStatus::Ready(pipeline)), 
                    Some(ResourceStatus::Ready(u_material)),
                    Some(ResourceStatus::Ready(u_instance))) = (geo_status, pip_status, u_mat_status, u_instance_status) 
            {
                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(MATERIAL_UNIFORMS, u_material, &[]);
                render_pass.set_vertex_buffer(0, geometry.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, u_instance.slice(..));
                render_pass.set_index_buffer(geometry.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..geometry.num_indices, 0, 0..insance_cmd.instances);
            }
        }
    }
}