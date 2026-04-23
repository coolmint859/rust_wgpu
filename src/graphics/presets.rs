#![allow(dead_code)]
use std::{collections::HashMap, sync::Arc};

use crate::graphics::{
   material::{ColorComponent, Material, SamplerComponent, TextureComponent},
   render_pipeline::RenderPipelineBuilder, 
   texture::SamplerBuilder, 
   vertex::{VertexAttribute, VertexLayoutBuilder}
};

/// preset material configurations
pub enum MaterialPreset {
    /// A material with a single color uniform
    ColoredSprite ([f32; 4]),
    /// A material with a single texture uniform (and sampler)
    TexturedSprite (&'static str),
}

impl MaterialPreset {
    pub fn with_label(self, label: &str) -> Arc<Material> {
        match self {
            MaterialPreset::ColoredSprite(color) => {
                let mut map = HashMap::new();
                map.insert(label.to_string(), 0);

                let mut material = Material::new("colored-sprite", map.clone());
                let _ = material.add_component(ColorComponent::new(label, color));

                Arc::new(material)
            }
            MaterialPreset::TexturedSprite(path) => {
                let mut map = HashMap::new();
                map.insert(label.to_string(), 0);
                map.insert(TextureSampler::NearestClampToEdge.as_key(), 1);

                let mut material = Material::new("textured-sprite", map.clone());
                let _ = material.add_component(TextureComponent::new(label, path));
                let _ = material.add_component(SamplerComponent::new(TextureSampler::NearestClampToEdge).with_bind_slot(1));

                Arc::new(material)
            }
        }
    }
}

/// Preset rendering pipelines
pub enum RenderPipeline {
    /// Simple 2D colored sprite rendering pipeline
    ColoredSprite,
    /// 2D textured sprite rendering pipeline
    TexturedSprite,
    /// 2D colored sprite pipeline for multiple instances
    ColoredSpriteInstanced,
    /// 2D textured sprite pipeline for multiple instances
    TexturedSpriteInstanced,
}

impl RenderPipeline {
    /// Get the RenderPipelineBuilder that this RenderPipeline represents
    pub fn get(self) -> RenderPipelineBuilder {
        return match self {
            RenderPipeline::ColoredSprite => {
                let path = "src/graphics/shaders/colored_sprite.wgsl";
                let vertex_builder = VertexLayoutBuilder::with_position();

                RenderPipelineBuilder::new(path, 3, vertex_builder).with_label("colored-sprite")
            }
            RenderPipeline::TexturedSprite => {
                let path = "src/graphics/shaders/textured_sprite.wgsl";
                let vertex_builder = VertexLayoutBuilder::with_position().with_attribute(VertexAttribute::UV);

                RenderPipelineBuilder::new(path, 3, vertex_builder)
                    .with_label("textured-sprite")
                    .with_alpha_blending()
            }
            RenderPipeline::ColoredSpriteInstanced => {
                let path = "src/graphics/shaders/colored_sprite_instanced.wgsl";
                let vertex_builder = VertexLayoutBuilder::with_position();
                let transform_builder = VertexLayoutBuilder::with_transform();

                RenderPipelineBuilder::new(path, 2, vertex_builder)
                    .with_label("colored-sprite-instanced")
                    .with_vertex_layout(transform_builder)
            },
            RenderPipeline::TexturedSpriteInstanced => {
                let path = "src/graphics/shaders/textured_sprite_instanced.wgsl";
                let vertex_builder = VertexLayoutBuilder::with_position().with_attribute(VertexAttribute::UV);
                let transform_builder = VertexLayoutBuilder::with_transform();

                RenderPipelineBuilder::new(path, 2, vertex_builder)
                    .with_label("textured-sprite-instanced")
                    .with_vertex_layout(transform_builder)
                    .with_custom_blending( wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        }
                    })
            }
        }
    }
}

/// Represents a sampler with a specific address and filter mode, as supported by wgpu
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum TextureSampler {
    NearestClampToEdge,
    NearestClampToBorder,
    NearestRepeat,
    NearestMirrorRepeat,
    LinearClampToEdge,
    LinearClampToBorder,
    LinearRepeat,
    LinearMirrorRepeat,
}

impl TextureSampler {
    /// Get the SamplerBuilder that this TextureSampler represents
    pub fn get(self) -> SamplerBuilder {
        match self {
            TextureSampler::NearestClampToEdge => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToEdge, wgpu::FilterMode::Nearest)
                    .with_label(&TextureSampler::NearestClampToEdge.as_key())
            },
            TextureSampler::NearestClampToBorder => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToBorder, wgpu::FilterMode::Nearest)
                    .with_label(&TextureSampler::NearestClampToBorder.as_key())
            },
            TextureSampler::NearestRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::Repeat, wgpu::FilterMode::Nearest)
                    .with_label(&TextureSampler::NearestRepeat.as_key())
            },
            TextureSampler::NearestMirrorRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::MirrorRepeat, wgpu::FilterMode::Nearest)
                    .with_label(&TextureSampler::NearestMirrorRepeat.as_key())
            },
            TextureSampler::LinearClampToEdge => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToEdge, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearClampToEdge.as_key())
            },
            TextureSampler::LinearClampToBorder => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToBorder, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearClampToBorder.as_key())
            }
            TextureSampler::LinearRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::Repeat, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearRepeat.as_key())
            },
            TextureSampler::LinearMirrorRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::MirrorRepeat, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearMirrorRepeat.as_key())
            },
        }
    }

    /// Get this sampler as it's key name
    pub fn as_key(self) -> String {
        match self {
            TextureSampler::LinearClampToBorder => "linear_clamp-to-border".to_string(),
            TextureSampler::LinearClampToEdge => "linear_clamp-to-edge".to_string(),
            TextureSampler::LinearMirrorRepeat => "linear_mirror-repeat".to_string(),
            TextureSampler::LinearRepeat => "linear_repeat".to_string(),
            TextureSampler::NearestClampToBorder => "nearest_clamp-to-border".to_string(),
            TextureSampler::NearestClampToEdge => "nearest_clamp-to-edge".to_string(),
            TextureSampler::NearestRepeat => "nearest_repeat".to_string(),
            TextureSampler::NearestMirrorRepeat => "nearest_mirror-repeat".to_string(),
        }
    }
}
