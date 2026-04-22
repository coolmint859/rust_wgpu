#![allow(dead_code)]
use std::sync::Arc;

use bytemuck::NoUninit;
use image::GenericImageView;
use super::handler::ResourceBuilder;

/// Simple struct representing a pixel in a texture
#[repr(C)]
#[derive(Clone, Copy, NoUninit)]
pub struct Pixel {
    r: u8, g: u8, b: u8, a: u8
}
const MAGENTA_PIX: Pixel = Pixel { r: 255, g: 0, b: 255, a: 255};
const WHITE_PIX: Pixel = Pixel { r: 255, g: 255, b: 255, a: 255};

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct SamplerBuilder {
    label: String,
    address_mode: wgpu::AddressMode, // e.g., Repeat, ClampToEdge
    filter_mode: wgpu::FilterMode,        // e.g., Linear, Nearest
}

impl SamplerBuilder {
    pub fn new(address_mode: wgpu::AddressMode, filter_mode: wgpu::FilterMode) -> Self {
        Self {
            label: "sampler".to_string(),
            address_mode,
            filter_mode,
        }
    }

    /// Add a custom label for GPU profiling
    pub fn with_label(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }
}

impl ResourceBuilder for SamplerBuilder {
    type Output = Arc<wgpu::Sampler>;
    type Context = wgpu::Device;

    fn build(&self, device: Arc<wgpu::Device>) -> Result<Arc<wgpu::Sampler>, String> {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: self.address_mode,
            address_mode_v: self.address_mode,
            mag_filter: self.filter_mode,
            min_filter: self.filter_mode,
            ..Default::default()
        });
        
        println!("[Sampler] Created new sampler with label '{}'", self.label);

        Ok(Arc::new(sampler))
    }
}

/// Houses the environment needed to construct textures
#[derive(Clone)]
pub struct TextureContext {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb
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

    /// get the raw 
    fn get_img_raw(&self) -> (u32, u32, Vec<u8>) {
        self.img_path.clone()
            .and_then(|path| {
                match image::open(path.clone()) {
                    Ok(image) => {
                        let (w, h) = image.dimensions();
                        return Some((w, h, image.to_rgba8().into_raw()));
                    }
                    Err(e) => {
                        println!("[Texture Builder] An error occured when reading image file '{}': {}.", path, e);
                        return None;
                    }
                }
            })
            .or_else(|| {
                if let Some((w, h, data)) = self.data.clone() {
                    return Some((w, h, data));
                }
                return None;
            })
            .unwrap_or_else(|| {
                let pixels = vec![MAGENTA_PIX, WHITE_PIX, WHITE_PIX, MAGENTA_PIX];
                return (2, 2, bytemuck::cast_slice(&pixels).to_vec())
            })
    }
}

impl ResourceBuilder for TextureBuilder {
    type Output = Arc<wgpu::TextureView>;
    type Context = TextureContext;

    fn build(&self, context: Arc<TextureContext>) -> Result<Self::Output, String> {
        let (width, height, data) = self.get_img_raw();

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

        println!("[Texture View] Created new texture view with label '{}'", self.label);

        Ok(Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default())))
    }
}