#![allow(dead_code)]
use std::sync::Arc;

use super::gpu_resource::ResourceBuilder;

#[derive(Clone, Debug)]
pub struct TextureBuilder {
    key: String,
    label: String,
    img_path: Option<String>,
}

impl TextureBuilder {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            label: "texture".to_string(),
            img_path: None
        }
    }

    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    pub fn with_img_file(mut self, img_path: &str) -> Self {
        self.img_path = Some(img_path.to_string());
        self
    }

    // pub fn with_bytes(mut self, )
}

impl ResourceBuilder for TextureBuilder {
    type Output = wgpu::Texture;
    type Context = wgpu::Device;

    fn build(&self, _device: Arc<wgpu::Device>) -> Result<Self::Output, String> {
        todo!();
    }
}