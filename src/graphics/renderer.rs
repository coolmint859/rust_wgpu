#![allow(dead_code)]
pub enum RenderCommand {
    Mesh,
    Sprite,
    Pass,
}

pub struct Renderer {
    pub command_queue: Vec<RenderCommand>
}

impl Renderer {
    pub fn new() -> Self {
        Self { command_queue: Vec::new() }
    }
}