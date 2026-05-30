#![allow(dead_code)]

use glam::Vec4;

use crate::graphics::{ data_table::{DataTable, DirtyVec}, entity::Entity, instance::{TINT_ATTR, UV_BOUNDS_ATTR}};

/// Represents animation behaviors on an entity orchestrated by an AnimationController
pub trait Animation {
    /// Advance the animation and update the instance attributes
    /// 
    /// * 'instances' - the set of instance data to update based on the animation
    /// * 'curr_frame' - the current animation frame
    /// * 'et' - the total elapsed time
    fn update(&mut self, instances: &mut DataTable, curr_frame: usize, et: f32);
}

/// Represents systems that control animated entities
pub trait AnimationController {
    /// advance the animation and animate the provided instances
    fn animate(&mut self, entity: &mut Entity, dt: f32, et: f32);
}

/// An animator that animates entities in a loop.
pub struct CyclicAnimator {
    /// the set of associated animations
    animations: Vec<Box<dyn Animation>>,
    /// the amount of time between animation frames
    frame_times: Vec<f32>,
    /// The current animation frame
    current_frame: usize,
    /// the timer for individual frames
    frame_timer: f32,
}

impl CyclicAnimator {
    pub fn new(frame_times: Vec<f32>) -> Self {
        Self {
            animations: Vec::new(),
            current_frame: 0,
            frame_times,
            frame_timer: 0.0,
        }
    }

    /// Add an animation to this animation controller
    pub fn with_animation(mut self, animation: impl Animation + 'static) -> Self {
        self.animations.push(Box::new(animation));
        self
    }

    /// Add an animation to this animation controller
    pub fn add_animation(&mut self, animation: impl Animation + 'static) {
        self.animations.push(Box::new(animation));
    }
}

impl AnimationController for CyclicAnimator {
    fn animate(&mut self, entity: &mut Entity, dt: f32, et: f32) {
        self.frame_timer += dt;

        if self.frame_timer >= self.frame_times[self.current_frame] { 
            self.frame_timer -= self.frame_times[self.current_frame];
            self.current_frame = (self.current_frame + 1) % self.frame_times.len();
        }
        
        for anim in self.animations.iter_mut() {
            anim.update(entity.instances.get_instances_mut(), self.current_frame, et);
        }
    }
}

/// Constructs equally spaced uv offsets and scales for use in a TextureComponent
pub struct UniformSpriteSheet {
    frames: Vec<Vec4>,
}

impl UniformSpriteSheet {
    fn new(cols: usize, rows: usize) -> Self {
        let mut frames = Vec::with_capacity(cols * rows);

        let scale_x = 1.0 / cols as f32;
        let scale_y = 1.0 / rows as f32;

        for r in 0..rows {
            for c in 0..cols {
                let offset_x = (c as f32) * scale_x;
                let offset_y = (r as f32) * scale_y;

                frames.push(Vec4::new(offset_x, offset_y, scale_x, scale_y));
            }
        }

        Self { frames }
    }

    /// Get the bounds stored at the given index
    fn get(&self, idx: usize) -> &Vec4 {
        self.frames.get(idx).unwrap()
    }
}

/// Animates a texture by updating the uv bounds based on a sprite sheet.
pub struct TextureAnimation  {
    sheet: UniformSpriteSheet,
    last_frame: usize,
}

impl TextureAnimation {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { 
            sheet: UniformSpriteSheet::new(cols, rows),
            last_frame: 0,
        }
    }
}

impl Animation for TextureAnimation {
    fn update(&mut self, instances: &mut DataTable, curr_frame: usize, _et: f32) {
        if self.last_frame == curr_frame { return; }
        self.last_frame = curr_frame;

        if let Some(uv_bounds) = instances.get_property_mut::<DirtyVec<Vec4>>(UV_BOUNDS_ATTR) {
            for bounds in uv_bounds.as_vec_mut().iter_mut() {
                bounds.set(*self.sheet.get(self.last_frame));
            }
        }
    }
}

/// Determines how a entity fades
pub enum FadeMode {
    /// Linearly increase the alpha value to 1
    Increase,
    /// Linearly decrease the alpha value to 0
    Decrease,
    /// Update the alpha value according to a square wave
    Square,
    /// Update the alpha value according to a sine wave with the provided phase
    Sinusoidal(f32)
}

impl FadeMode {
    /// Get the alpha value for a fade antimation based on the duration and elapsed time
    pub fn get_alpha(&self, et: f32, duration: f32) -> f32 {
        if duration <= 0.0 { return 1.0 }

        let progress = (et % duration) / duration;
        match self {
            FadeMode::Increase => progress.min(1.0),
            FadeMode::Decrease => (1.0 - progress).max(0.0),
            FadeMode::Square => {
                if progress >= 0.5 { return 1.0 }
                else { return 0.0 };
            }
            FadeMode::Sinusoidal(phase) => {
                let angle = progress * std::f32::consts::TAU;
                ((angle + phase).sin() * 0.5) + 0.5
            }
        }
    }
}

/// Fades an entity according to a mode for a specified duration
pub struct FadeAnimation {
    mode: FadeMode,
    duration: f32,
}

impl FadeAnimation {
    pub fn new(mode: FadeMode, duration: f32) -> Self {
        Self { mode, duration }
    }
}

impl Animation for FadeAnimation {
    fn update(&mut self, instances: &mut DataTable, _curr_frame: usize, et: f32) {
        let alpha = self.mode.get_alpha(et, self.duration);

        if let Some(tints) = instances.get_property_mut::<DirtyVec<Vec4>>(TINT_ATTR) {
            for tint in tints.as_vec_mut().iter_mut() {
                tint.w = alpha;
            }
        }
    }
}
