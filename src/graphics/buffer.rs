use std::sync::Arc;
use wgpu::util::DeviceExt;

use super::mesh::Mesh;
use super::traits::{ Handler, ResourceDescriptor };
use super::registry::ResourceRegistry;

/// represents a mesh as it lives on the gpu during rendering, most importantly it's buffers
pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl GpuMesh {
    pub async fn new(device: Arc<wgpu::Device>, mesh: Mesh) -> Result<GpuMesh, String> {
        let mesh_id = mesh.get_key().clone();

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("Mesh[id={}] Vertex Buffer", mesh_id).as_str()),
                contents: bytemuck::cast_slice(mesh.vertex_data().as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("Mesh[id={}] Index Buffer", mesh_id).as_str()),
                contents: bytemuck::cast_slice(mesh.index_data().as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Ok(Self {
            vertex_buffer,
            index_buffer,
            num_indices: mesh.num_indices(),
        })
    }
}


/// Container for Mesh Buffers stored in GPU memory.
/// 
/// Stores references to buffers that can be requested by id during runtime for hot reloading
pub struct MeshBufferHandler {
    device: Arc<wgpu::Device>,
    registry: ResourceRegistry<u32, GpuMesh>,
}

impl MeshBufferHandler {
    pub fn new(device: &Arc<wgpu::Device>) -> Self {
        Self {
            device: Arc::clone(&device),
            registry: ResourceRegistry::new(),
        }
    }
}

impl Handler<Mesh, GpuMesh> for MeshBufferHandler {
    fn request_new(&mut self, mesh: &Mesh) {
        let mesh_cpy = mesh.clone();
        let device_cpy = Arc::clone(&self.device);
        self.registry.request_new(mesh.get_key(), GpuMesh::new(device_cpy, mesh_cpy));
    }

    fn sync(&mut self) {
        self.registry.sync();
    }

    fn get(&self, mesh_id: &u32) -> Option<&GpuMesh> {
        self.registry.get(mesh_id)
    }

    fn remove(&mut self, mesh_id: &u32) {
        self.registry.remove(mesh_id);
    }
}