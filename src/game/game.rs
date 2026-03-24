#![allow(dead_code)]
use super::super::graphics::renderer::Renderer;
use super::super::graphics::traits::AppState;

pub struct Game {

}

impl Game {
    pub fn new() -> Self {
        Self {  }
    }
}

impl AppState for Game {
    fn init(&mut self) {

    }

    fn process_input(&mut self) {
        
    }

    fn update(&mut self, _dt: f32) {
        
    }

    fn render(&mut self, _renderer: &Renderer) {
        
    }
}