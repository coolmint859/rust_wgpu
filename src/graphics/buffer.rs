use std::{collections::HashMap, sync::{Arc, mpsc}};
use wgpu::util::DeviceExt;
use tokio::task;

use super::mesh::Mesh;

/// represents a mesh as it lives on the gpu during rendering, most importantly it's buffers
pub struct GpuMesh {
    pub mesh_id: u32,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl GpuMesh {
    pub async fn new(device: Arc<wgpu::Device>, mesh: &Mesh) -> Self {
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("Mesh[id={}] Vertex Buffer", mesh.id()).as_str()),
                contents: bytemuck::cast_slice(mesh.vertex_data().as_slice()),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some(format!("Mesh[id={}] Index Buffer", mesh.id()).as_str()),
                contents: bytemuck::cast_slice(mesh.index_data().as_slice()),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        Self {
            mesh_id: mesh.id(),
            vertex_buffer,
            index_buffer,
            num_indices: mesh.num_indices(),
        }
    }
}

pub struct BufferHandler {
    device: Arc<wgpu::Device>,
    gpu_meshes: HashMap<u32, Option<GpuMesh>>,
    tx: mpsc::Sender<GpuMesh>,
    rx: mpsc::Receiver<GpuMesh>,
}

impl BufferHandler {
    pub fn new(device: &Arc<wgpu::Device>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            device: Arc::clone(&device),
            gpu_meshes: HashMap::new(),
            tx, rx
        }
    }

    pub fn request_gpu_mesh(&mut self, mesh: &Mesh) {
        if self.gpu_meshes.contains_key(&mesh.id()) {
            return; // gpu mesh already requested
        }

        let device = Arc::clone(&self.device);
        let tx = self.tx.clone();
        let cpu_mesh = mesh.clone();

        self.gpu_meshes.insert(mesh.id(), None);

        // have the mesh buffers be created in a separate thread
        task::spawn(async move {
            let gpu_mesh = GpuMesh::new(device, &cpu_mesh).await;
            let _ = tx.send(gpu_mesh);
        });
    }

    pub fn check_ready_buffers(&mut self) {
        while let Ok(gpu_mesh) = self.rx.try_recv() {
            self.gpu_meshes.insert(gpu_mesh.mesh_id, Some(gpu_mesh));
        }
    }

    pub fn get_gpu_mesh(&self, mesh_id: u32) -> Option<&GpuMesh> {
        return self.gpu_meshes.get(&mesh_id)?.as_ref();
    }
}