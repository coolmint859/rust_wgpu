#![allow(dead_code)]
use crate::graphics::{
   bind_group::{BindGroupLayoutBuilder, LayoutBindType, LayoutEntry, LayoutVisibility}, material::{ColorComponent, Material, SamplerComponent, TextureComponent}, render_pipeline::RenderPipelineBuilder, shader::ShaderSpecBuilder, texture::SamplerBuilder, vertex::{POSITION_LOC, TINT_LOC, TRANSFORM_LOC, UV_BOUNDS_LOC, UV_LOC}
};

/// preset material configurations
pub enum MaterialPreset {
    /// A material with a single color uniform
    ColoredSprite ([f32; 4]),
    /// A material with a single texture uniform (and sampler)
    TexturedSprite (String),
}

impl MaterialPreset {
    pub fn with_label(self, label: &str) -> Material {
        match self {
            MaterialPreset::ColoredSprite(color) => {
                let mut material = Material::new("colored-sprite");
                material.add_component(ColorComponent::new(label, color));

                material
            }
            MaterialPreset::TexturedSprite(path) => {
                let mut material = Material::new("textured-sprite");
                material.add_component(TextureComponent::new(label, &path));
                material.add_component(SamplerComponent::new(TextureSampler::NearestClampToEdge));

                material
            }
        }
    }
}

pub enum ShaderSpecPreset {
    ColoredSprite,
    TexturedSprite,
    AnimatedSprite,
}

impl ShaderSpecPreset {
    pub fn get(self) -> ShaderSpecBuilder {
        let global_bg_layout = BindGroupLayoutBuilder::new()
            .with_label("global-uniforms")
            .with_entry(LayoutEntry { 
                binding: 0, 
                visibility: LayoutVisibility::VertexFragment, 
                ty: LayoutBindType::Uniform 
            });

        let textured_sprite_layout = BindGroupLayoutBuilder::new()
            .with_label("textured-sprite")
            .with_entry(LayoutEntry {
                binding: 0,
                visibility: LayoutVisibility::Fragment,
                ty: LayoutBindType::Texture,
            })
            .with_entry(LayoutEntry { 
                binding: 1, 
                visibility: LayoutVisibility::Fragment, 
                ty: LayoutBindType::Sampler 
            });

        match self {
            ShaderSpecPreset::ColoredSprite => {
                let colored_sprite_layout = BindGroupLayoutBuilder::new()
                    .with_label("colored-sprite")
                    .with_entry(LayoutEntry {
                        binding: 0,
                        visibility: LayoutVisibility::Fragment,
                        ty: LayoutBindType::Uniform,
                    });

                ShaderSpecBuilder::new(&self.path())
                    .with_vertex_attribute(POSITION_LOC, wgpu::VertexFormat::Float32x3)
                    .with_instance_attribute(TRANSFORM_LOC, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+1, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+2, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+3, wgpu::VertexFormat::Float32x4)
                    .with_bg_layout(global_bg_layout)
                    .with_bg_layout(colored_sprite_layout)
            },
            ShaderSpecPreset::TexturedSprite => {
                ShaderSpecBuilder::new(&self.path())
                    .with_vertex_attribute(POSITION_LOC, wgpu::VertexFormat::Float32x3)
                    .with_vertex_attribute(UV_LOC, wgpu::VertexFormat::Float32x2)
                    .with_instance_attribute(TRANSFORM_LOC, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+1, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+2, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+3, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TINT_LOC, wgpu::VertexFormat::Float32x4)
                    .with_bg_layout(global_bg_layout)
                    .with_bg_layout(textured_sprite_layout)
            },
            ShaderSpecPreset::AnimatedSprite => {
                ShaderSpecBuilder::new(&self.path())
                    .with_vertex_attribute(POSITION_LOC, wgpu::VertexFormat::Float32x3)
                    .with_vertex_attribute(UV_LOC, wgpu::VertexFormat::Float32x2)
                    .with_instance_attribute(TRANSFORM_LOC, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+1, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+2, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TRANSFORM_LOC+3, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(TINT_LOC, wgpu::VertexFormat::Float32x4)
                    .with_instance_attribute(UV_BOUNDS_LOC, wgpu::VertexFormat::Float32x4)
                    .with_bg_layout(global_bg_layout)
                    .with_bg_layout(textured_sprite_layout)
            }
        }
    }

    pub fn path(&self) -> String {
        match self {
            ShaderSpecPreset::ColoredSprite => "src/graphics/shaders/colored_sprite.wgsl".to_string(),
            ShaderSpecPreset::TexturedSprite => "src/graphics/shaders/textured_sprite.wgsl".to_string(),
            ShaderSpecPreset::AnimatedSprite => "src/graphics/shaders/animated_sprite.wgsl".to_string(),
        }
    }

    pub fn from_known_path(shader_path: &String) -> Option<ShaderSpecBuilder> {
        if shader_path == &ShaderSpecPreset::ColoredSprite.path() {
            Some(ShaderSpecPreset::ColoredSprite.get())
        } else if shader_path == &ShaderSpecPreset::TexturedSprite.path() {
            Some(ShaderSpecPreset::TexturedSprite.get())
        } else if shader_path == &ShaderSpecPreset::AnimatedSprite.path() {
            Some(ShaderSpecPreset::AnimatedSprite.get())
        } else {
            None
        }
    }
}

/// Preset rendering pipelines
pub enum RenderPipeline {
    /// 2D colored sprite pipeline
    ColoredSprite,
    /// 2D textured sprite pipeline
    TexturedSprite,
    /// 2D animated sprite pipeline
    AnimatedSprite,
}

impl RenderPipeline {
    /// Get the RenderPipelineBuilder that this RenderPipeline represents
    pub fn get(self) -> RenderPipelineBuilder {
        return match self {
            RenderPipeline::ColoredSprite => {
                RenderPipelineBuilder::new().with_label("colored-sprite")
            }
            RenderPipeline::TexturedSprite => {
                RenderPipelineBuilder::new().with_label("textured-sprite")
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
            },
            RenderPipeline::AnimatedSprite => {
                RenderPipelineBuilder::new().with_label("animated-sprite")
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
                    .with_label(&TextureSampler::NearestClampToEdge.label())
            },
            TextureSampler::NearestClampToBorder => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToBorder, wgpu::FilterMode::Nearest)
                    .with_label(&TextureSampler::NearestClampToBorder.label())
            },
            TextureSampler::NearestRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::Repeat, wgpu::FilterMode::Nearest)
                    .with_label(&TextureSampler::NearestRepeat.label())
            },
            TextureSampler::NearestMirrorRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::MirrorRepeat, wgpu::FilterMode::Nearest)
                    .with_label(&TextureSampler::NearestMirrorRepeat.label())
            },
            TextureSampler::LinearClampToEdge => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToEdge, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearClampToEdge.label())
            },
            TextureSampler::LinearClampToBorder => {
                SamplerBuilder::new(wgpu::AddressMode::ClampToBorder, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearClampToBorder.label())
            }
            TextureSampler::LinearRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::Repeat, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearRepeat.label())
            },
            TextureSampler::LinearMirrorRepeat => {
                SamplerBuilder::new(wgpu::AddressMode::MirrorRepeat, wgpu::FilterMode::Linear)
                    .with_label(&TextureSampler::LinearMirrorRepeat.label())
            },
        }
    }

    /// Get this sampler as it's key name
    pub fn label(&self) -> String {
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
