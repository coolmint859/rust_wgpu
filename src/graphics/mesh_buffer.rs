use std::sync::Arc;
use wgpu::util::DeviceExt;

use super::mesh::MeshData;
use super::traits::{ Handler, ResourceDescriptor };
use super::registry::ResourceRegistry;

/// represents a mesh as it lives on the gpu during rendering, most importantly it's buffers
pub struct GpuMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl GpuMesh {
    pub async fn new(device: Arc<wgpu::Device>, mesh: Arc<MeshData>) -> Result<GpuMesh, String> {
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

impl Handler<MeshData, GpuMesh> for MeshBufferHandler {
    fn request_new(&mut self, mesh_data: Arc<MeshData>) {
        let mesh_cpy = Arc::clone(&mesh_data);
        let device_cpy = Arc::clone(&self.device);
        self.registry.request_new(mesh_data.get_key(), GpuMesh::new(device_cpy, mesh_cpy));
    }

    fn request_wait(&mut self, mesh_data: Arc<MeshData>) {
        let device_cpy = Arc::clone(&self.device);
        let mesh_cpy = Arc::clone(&mesh_data);

        let result = self.registry.request_wait(
            mesh_data.get_key(),
            GpuMesh::new(device_cpy, mesh_cpy),
        );

        match result {
            Err(e) => eprintln!("Error creating mesh buffers: {e}"),
            _ => {}
        }
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

    fn contains(&self, mesh_id: &u32) -> bool {
        return self.registry.contains(mesh_id);
    }

    fn is_ready(&self, mesh_id: &u32) -> bool {
        return self.registry.is_ready(mesh_id);
    }

    fn is_pending(&self, mesh_id: &u32) -> bool {
        return self.registry.is_pending(mesh_id);
    }

    fn is_failed(&self, mesh_id: &u32) -> bool {
        return self.registry.is_failed(mesh_id);
    }

    fn get_err(&self, mesh_id: &u32) -> Option<&str> {
        return self.registry.get_err(mesh_id);
    }
}