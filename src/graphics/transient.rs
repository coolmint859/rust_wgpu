#![allow(dead_code)]

use std::sync::Arc;

use super::traits::CommandBuffer;

use super::mesh::Mesh;
use super::presets::RenderPipelineConfig;

#[derive(Clone)]
pub enum RenderCommand {
    Mesh(Arc<Mesh>, Arc<RenderPipelineConfig>)
}

/// Command Buffer for draw calls
pub struct Renderer {
    commands: Vec<RenderCommand>,
    clear_color: wgpu::Color,
}

impl Renderer {
    pub fn new() -> Self {
        Self { 
            commands: Vec::new(),
            clear_color: wgpu::Color::BLACK
        }
    }

    // set the background color for the frame
    pub fn set_bg_color(&mut self, r: f64, g: f64, b: f64) {
        self.clear_color = wgpu::Color { r, g, b, a: 1.0 }
    }

    // get the currently set background color (default is black)
    pub fn get_bg_color(&self) -> &wgpu::Color {
        &self.clear_color
    }

    /// draw an object to the screen via a RenderCommand
    pub fn draw(&mut self, cmd: RenderCommand) {
        self.commands.push(cmd);
    }
}

impl CommandBuffer<RenderCommand> for Renderer {
    fn get_commands(&self) -> Vec<RenderCommand> {
        self.commands.to_vec()
    }
}

#[derive(Clone)]
pub enum InitCommand {
    Mesh(Arc<Mesh>),
    Pipeline(Arc<RenderPipelineConfig>),
}

/// Command Buffer for app state initialization
pub struct StateInit {
    commands: Vec<InitCommand>,
}

impl StateInit {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Add a pipeline to the initialization commands. 
    /// 
    /// Note: these are considered 'immediate', and will block the event loop until the pipeline is created.
    pub fn add_render_pipeline(&mut self, desc: Arc<RenderPipelineConfig>) {
        self.commands.push(InitCommand::Pipeline(desc));
    }

    /// Add a mesh to the initialization commands. 
    /// 
    /// Note: these are considered 'immediate', and will block the event loop until the mesh is created.
    pub fn add_mesh(&mut self, mesh: Arc<Mesh>) {
        self.commands.push(InitCommand::Mesh(mesh));
    }
}

impl CommandBuffer<InitCommand> for StateInit {
    fn get_commands(&self) -> Vec<InitCommand> {
        self.commands.to_vec()
    }
}
