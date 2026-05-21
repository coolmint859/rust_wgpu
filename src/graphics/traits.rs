#![allow(dead_code)]
use crate::graphics::{
    init_state::StateInit, 
    renderer::Renderer
};

pub trait Driver {
    /// Called when the app changes to this state; intitializes this state of the app
    fn init(&mut self, state_init: &mut StateInit);

    /// Called at the beginning of each frame to process any input gathered
    fn process_input(&mut self, dt: f32, et: f32);

    /// Called once each frame; should be used to update internal state
    fn update(&mut self, dt: f32, et: f32);

    /// Called at the end of each frame before drawing commands are sent to the GPU.
    fn render(&mut self, renderer: &mut Renderer, aspect: f32);
}

/// Represents systems that control animated entities
pub trait AnimationController {
    /// Advance the animation timeline according to dt
    fn update(&mut self, dt: f32);

    /// render the animation.
    fn render(&mut self, renderer: &mut Renderer);
}