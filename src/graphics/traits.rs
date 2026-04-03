#![allow(dead_code)]
use crate::graphics::{
    registry::ResourceStatus, 
    renderer::Renderer, 
    init_state::StateInit,
};

use std::{hash::Hash, sync::Arc};

pub trait AppState {
    /// Called when the app changes to this state; intitializes this state of the app
    fn init(&mut self, state_init: &mut StateInit);

    /// Called at the beginning of each frame to process any input gathered
    fn process_input(&mut self, dt: f32);

    /// Called once each frame; should be used to update internal state
    fn update(&mut self, dt: f32);

    /// Called at the end of each frame before drawing commands are sent to the GPU.
    fn render(&mut self, renderer: &mut Renderer, aspect: f32);
}

/// Represents a storable resource in a type that implements the Handler trait
/// 
/// Key type must implement the Hash, Eq, and Send traits.
pub trait ResourceDescriptor: Send + 'static {
    type Key: Hash + Eq + Send + 'static;

    /// get the key associated with this resource.
    fn get_key(&self) -> &Self::Key;
}

/// Handles resources used by the app through worker threads, typically GPU related
/// 
/// type C: descriptor for creating new resources
/// 
/// type R: a reference to a stored resource instance 
pub trait Handler<D, R>
where D: ResourceDescriptor
{
    /// Request creation of a new resource through it's descriptor; intitializes a worker thread
    fn request_new(&mut self, desc: Arc<D>);

    /// Request a resource and wait for its creation.
    /// 
    /// Note: This method blocks on the main thread.
    fn request_wait(&mut self, desc: Arc<D>);

    /// Syncronize the internal worker threads with the main thread, making available any completed resources. Should be called regularly
    fn sync(&mut self);

    /// Get a reference to a stored instance of a resource via its key; returns None if the resource does not exist/is unavailable
    fn get(&self, key: &D::Key) -> Option<&R>;

    // Remove a stored instance from the internal registry.
    fn remove(&mut self, key: &D::Key);

    /// Check if a resource exists in the handler (in any state)
    fn contains(&self, key: &D::Key) -> bool;

    /// Get the status of resource (Pending, Ready, Failed). Returns None if the resource does not exist.
    fn status_of(&self, key: &D::Key) -> Option<&ResourceStatus<R>>;
}