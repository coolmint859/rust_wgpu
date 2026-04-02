use std::sync::Arc;
use wgpu::util::DeviceExt;

use super::mesh::MeshData;
use super::traits::{ Handler, ResourceDescriptor };
use super::registry::{ ResourceStatus, ResourceRegistry};

/// represents a mesh as it lives on the gpu during rendering, most importantly it's buffers
pub struct MeshBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl MeshBuffer {
    pub async fn new(device: Arc<wgpu::Device>, mesh_data: Arc<MeshData>) -> Result<MeshBuffer, String> {
        let mesh_id = mesh_data.get_key().clone();

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("MeshData[id={}] Vertex Buffer", mesh_id).as_str()),
                contents: bytemuck::cast_slice(mesh_data.vertex_data().as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("MeshData[id={}] Index Buffer", mesh_id).as_str()),
                contents: bytemuck::cast_slice(mesh_data.index_data().as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        println!("Created vertex/index buffer '{}'", mesh_id);

        Ok(Self {
            vertex_buffer,
            index_buffer,
            num_indices: mesh_data.num_indices(),
        })
    }
}

/// Container for Mesh Buffers stored in GPU memory.
/// 
/// Stores references to buffers that can be requested by id during runtime for hot reloading
pub struct MeshBufferHandler {
    device: Arc<wgpu::Device>,
    registry: ResourceRegistry<u32, MeshBuffer>,
}

impl MeshBufferHandler {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        Self {
            device: device,
            registry: ResourceRegistry::new(),
        }
    }
}

impl Handler<MeshData, MeshBuffer> for MeshBufferHandler {
    fn request_new(&mut self, mesh_data: Arc<MeshData>) {
        let mesh_cpy = Arc::clone(&mesh_data);
        let device_cpy = Arc::clone(&self.device);
        self.registry.request_new(mesh_data.get_key(), MeshBuffer::new(device_cpy, mesh_cpy));
    }

    fn request_wait(&mut self, mesh_data: Arc<MeshData>) {
        let device_cpy = Arc::clone(&self.device);
        let mesh_cpy = Arc::clone(&mesh_data);

        let result = self.registry.request_wait(
            mesh_data.get_key(),
            MeshBuffer::new(device_cpy, mesh_cpy),
        );

        match result {
            Err(e) => eprintln!("Error creating mesh buffers: {e}"),
            _ => {}
        }
    }

    fn sync(&mut self) {
        self.registry.sync();
    }

    fn get(&self, mesh_id: &u32) -> Option<&MeshBuffer> {
        self.registry.get(mesh_id)
    }

    fn remove(&mut self, mesh_id: &u32) {
        self.registry.remove(mesh_id);
    }

    fn contains(&self, mesh_id: &u32) -> bool {
        return self.registry.contains(mesh_id);
    }

    fn status_of(&self, mesh_id: &u32) -> Option<&ResourceStatus<MeshBuffer>> {
        self.registry.status_of(mesh_id)
    }
}