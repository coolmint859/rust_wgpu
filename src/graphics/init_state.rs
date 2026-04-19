#![allow(dead_code)]
use crate::graphics::{
    bind_group::BindGroupLayoutBuilder, 
    render_pipeline::RenderPipelineBuilder
};

#[derive(Clone, Debug)]
/// Initilazation mode for StateInit commands
pub enum InitMode {
    /// Request creation, and wait until finalized (blocks the main thread)
    Immediate,
    /// Request creation, but don't wait until finalized.
    Deferred
}

#[derive(Clone, Debug)]
/// Command for generating a render pipline
pub struct RenderPipelineCommand {
    pub builder: RenderPipelineBuilder,
    pub mode: InitMode,
}

#[derive(Clone, Debug)]
/// Command for generating a bind group layout
pub struct BindGroupLayoutCommand {
    pub builder: BindGroupLayoutBuilder,
    pub mode: InitMode,
}

/// Command Buffer for app state initialization
pub struct StateInit {
    rpip_commands: Vec<RenderPipelineCommand>,
    bgl_commands: Vec<BindGroupLayoutCommand>
}

impl StateInit {
    pub fn new() -> Self {
        Self {
            rpip_commands: Vec::new(),
            bgl_commands: Vec::new(),
        }
    }

    /// Add a render pipeline to the initialization commands.
    pub fn add_render_pipeline(&mut self, builder: RenderPipelineBuilder, mode: InitMode) {
        self.rpip_commands.push(RenderPipelineCommand { builder, mode });
    }

    /// Add a bind group layout to the initialization commands.
    pub fn add_bind_group_layout(&mut self, builder: BindGroupLayoutBuilder, mode: InitMode) {
        self.bgl_commands.push(BindGroupLayoutCommand { builder, mode })
    }

    pub fn get_rpip_cmds(&self) -> Vec<RenderPipelineCommand> {
        self.rpip_commands.to_vec()
    }

    pub fn get_bgl_cmds(&self) -> Vec<BindGroupLayoutCommand> {
        self.bgl_commands.to_vec()
    }
}