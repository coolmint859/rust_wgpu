#![allow(dead_code)]
use std::sync::Arc;

use crate::graphics::{
    mesh::Mesh, presets::Pipeline, traits::AppState, transient::{RenderCommand, Renderer, StateInit}, vertex::Vertex
};

pub struct Game {
    mesh: Arc<Mesh>,
}

impl Game {
    pub fn new() -> Self {
        // let shape = Mesh::new(
        //     vec![
        //         Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.5, 0.0, 0.5] }, // A
        //         Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.5] }, // B
        //         Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.5] }, // C
        //         Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.5, 0.0, 0.5] }, // D
        //         Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.5, 0.0, 0.5] }, // E
        //     ],
        //     vec![
        //         0, 1, 4,
        //         1, 2, 4,
        //         2, 3, 4
        //     ]
        // );

        let shape = Mesh::new(
            vec![
                Vertex { position: [0.5, 0.5, 0.0], color: [0.5, 0.0, 0.5] }, // A
                Vertex { position: [-0.5, 0.5, 0.0], color: [0.5, 0.0, 0.5] }, // B
                Vertex { position: [-0.5, -0.5, 0.0], color: [0.5, 0.0, 0.5] }, // C
                Vertex { position: [0.5, -0.5, 0.0], color: [0.5, 0.0, 0.5] }, // D
            ],
            vec![
                0, 1, 2,
                2, 3, 0
            ]
        );

        Self { mesh: Arc::new(shape) }
    }
}

impl AppState for Game {
    fn init(&mut self, state_init: &mut StateInit) {
        state_init.add_render_pipeline(Pipeline::ColoredSprite.instance());
        state_init.add_mesh(Arc::clone(&self.mesh));
    }

    fn process_input(&mut self) {
        
    }

    fn update(&mut self, _dt: f32) {
        
    }

    fn render(&mut self, renderer: &mut Renderer) {
        renderer.draw(
            RenderCommand::Mesh(Arc::clone(&self.mesh), 
            Pipeline::ColoredSprite.instance()),
        );
    }
}