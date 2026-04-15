#![allow(dead_code)]
use std::sync::Arc;

use image::GenericImageView;
use super::handler::ResourceBuilder;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct SamplerBuilder {
    pub address_mode: wgpu::AddressMode, // e.g., Repeat, ClampToEdge
    pub filter: wgpu::FilterMode,        // e.g., Linear, Nearest
}

impl ResourceBuilder for SamplerBuilder {
    type Output = Arc<wgpu::Sampler>;
    type Context = wgpu::Device;

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Arc<wgpu::Sampler>, String> {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: self.address_mode,
            address_mode_v: self.address_mode,
            mag_filter: self.filter,
            min_filter: self.filter,
            ..Default::default()
        });

        Ok(Arc::new(sampler))
    }
}

/// Houses the environment needed to construct textures
#[derive(Clone)]
pub struct TextureContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub sampler: Arc<wgpu::Sampler>
}

pub struct TextureResource {
    pub view: Arc<wgpu::TextureView>,
    pub sampler: Arc<wgpu::Sampler>
}

#[derive(Clone, Debug)]
pub struct TextureBuilder {
    label: String,
    img_path: Option<String>,
    data: Option<(u32, u32, Vec<u8>)>,
    format: wgpu::TextureFormat
}

impl TextureBuilder {
    pub fn new() -> Self {
        Self {
            label: "texture".to_string(),
            img_path: None,
            data: None,
            format: wgpu::TextureFormat::Bgra8Unorm
        }
    }
    
    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// Specify the format that this texture should be created with
    pub fn with_format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// Build a texture from an image file
    pub fn with_img_file(mut self, img_path: &str) -> Self {
        self.img_path = Some(img_path.to_string());
        self
    }

    /// Build a texture from raw bytes
    pub fn with_data(mut self, width: u32, height: u32, data: Vec<u8>) -> Self {
        self.data = Some((width, height, data));
        self
    }
}

impl ResourceBuilder for TextureBuilder {
    type Output = TextureResource;
    type Context = TextureContext;

    fn build(&self, context: Arc<TextureContext>) -> Result<Self::Output, String> {
        let (width, height, data) = if let Some(ref path) = self.img_path {
            let img = match image::open(path) {
                Ok(img) => img,
                Err(e) => { return Err(format!("Could not open image file: {e}")); }
            };

            let (w, h) = img.dimensions();
            (w, h, img.to_rgb8().into_raw())
        } else if let Some((w, h, ref data)) = self.data {
            (w, h, data.clone())
        } else {
            return Err("No valid data or image path specified for this TextureBuilder".to_string());
        };

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("TextureBuilder Result"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        context.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let sampler = context.sampler.clone();

        Ok(TextureResource { view, sampler })
    }
}