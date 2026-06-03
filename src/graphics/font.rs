use std::{collections::HashMap, fs::File, io::Read, sync::Arc};

use glam::Vec4;

use crate::graphics::{entity::{Entity, RenderInfo}, geometry::{Geometry, PositionAttribute, UVAttribute}, handler::ResourceBuilder, instance::{InstanceGroup, TintAttribute, TransformAttribute, UVBoundsAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, shape_factory::Shape2D, texture::{TextureBuilder, TextureContext}, transform::Transform};

#[derive(Clone, Debug)]
/// Data struct holding information about character glyphs for use during rendering
pub struct CharacterGlyph {
    /// The bounding box of the glyph in the atlas
    pub uv_bounds: Vec4,
    /// The layout bounds of the character relative to the cursor
    pub plane_bounds: Vec4,
    /// How far to advance the cursor after placing this character
    pub advance: f32,
}

/// The font glyph, atlas texture, and line heigh information used for rendering text.
#[derive(Clone, Debug)]
pub struct FontAsset {
    /// The handle to the gpu texture font atlas
    pub atlas: Arc<wgpu::TextureView>,
    /// The font glyph information used for rendering text
    pub glyphs: HashMap<char, CharacterGlyph>,
    /// The spacing between lines of text rendered with this font
    pub line_height: f32,
}

pub struct Font {
    /// the path to the ttf file
    pub path: String,
    /// the entity that is used to render text used with this font
    pub entity: Entity,
}

impl Font {
    pub fn new(path: &str) -> Self {
        let geometry = Geometry::new(Shape2D::new().square())
            .with_attribute(PositionAttribute)
            .with_attribute(UVAttribute)
            .with_attribute(UVBoundsAttribute);

        let group_capacity = 1000; // allow up to 1000 characters
        let instances = InstanceGroup::new(0, group_capacity) // allow space for 1000 characters
            .with_label(&format!("font::{}", path))
            .with_attribute(TransformAttribute, Vec::<Transform>::with_capacity(group_capacity))
            .with_attribute(TintAttribute, Vec::<Vec4>::with_capacity(group_capacity))
            .with_attribute(UVBoundsAttribute, Vec::<Vec4>::with_capacity(group_capacity));
        
        let label = format!("font::{}", path);
        let entity = Entity::from_group(
            &label, 
            geometry, 
            MaterialPreset::Font(path.to_string()).with_label(&label), 
            instances, 
            RenderInfo {
                shader_path: ShaderSpecPreset::Font.path(),
                pipeline: RenderPipeline::Font.get()
            },
        );

        Self {
            path: path.to_string(),
            entity,
        }
    }
}

/// Creates font atlases for use in rendering fonts
#[derive(Clone, Debug)]
pub struct FontBuilder {
    /// the path to the ttf file for the font
    path: String,
    /// the size (width and height) of the sdf font atlas in pixels
    atlas_size: u32, 
    /// the scale of the individual characters when rasterized in pixels per em
    font_scale: f32,
    /// the radius of the sdf gradient in the atlas
    sdf_radius: f32
}

impl FontBuilder {
    pub fn new(path: &str) -> Self {
        Self { 
            path: path.to_string(),
            atlas_size: 1024,
            font_scale: 64.0,
            sdf_radius: 8.0
        } 
    }
}

impl ResourceBuilder for FontBuilder {
    type Output = Arc<FontAsset>;
    type Context = TextureContext;

    fn build(&self, context: Arc<Self::Context>) -> Result<Self::Output, String> {
        let mut ttf_file = File::open(&self.path).map_err(|e| e.to_string())?;
        let mut font_data = Vec::new();
        ttf_file.read_to_end(&mut font_data).map_err(|e| e.to_string())?;

        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        let line_metrics = font.horizontal_line_metrics(self.font_scale)
            .ok_or("Failed to read font line metrics.")?;
        let line_height = line_metrics.new_line_size / self.font_scale;

        let (glyphs, atlas_bytes) = FontUtils::gen_sdf_font(
            font, 
            self.atlas_size,
            self.font_scale,
            self.sdf_radius
        );

        let atlas = TextureBuilder::new()
            .with_label(&self.path)
            .with_data(self.atlas_size, self.atlas_size, atlas_bytes)
            .with_format(wgpu::TextureFormat::R8Unorm)
            .build(context)?;

        let font_asset = FontAsset { atlas, glyphs, line_height };
        return Ok(Arc::new(font_asset));
    }
}

/// Helper functions for generating font atlases
pub struct FontUtils;
impl FontUtils {
    /// Generate an signed distance feild (sdf) font atlas by rasterizing a ttf font
    /// 
    /// * 'font' - the font containing the glyph metrics
    /// * 'size' - the size of the bitmap atlas in pixels
    /// * 'scale' - the size to rasterize the characters with, in pixels per em unit
    /// * 'radius' - spacing between bitmap characters
    pub fn gen_sdf_font(font: fontdue::Font, size: u32, scale: f32, radius: f32) -> (HashMap<char, CharacterGlyph>, Vec<u8>) {
        let mut atlas_bitmap = vec![0u8; (size * size) as usize];
        let mut glyphs = HashMap::new();

        let padding = radius as u32;
        let mut current_x: u32 = 0;
        let mut current_y: u32 = 0;
        let mut max_row_height: u32 = 0;

        for c in 32..128u8 {
            let character = c as char;
            let (metrics, bitmap) = font.rasterize(character, scale);

            // wrap the grid to the next row
            if current_x + metrics.width as u32 + (padding * 2) >= size {
                current_x = 0;
                current_y += max_row_height + (padding * 2);
                max_row_height = 0;
            }

            max_row_height = max_row_height.max(metrics.height as u32);

            let glyph_x = current_x + padding;
            let glyph_y = current_y + padding;

            if glyph_y + metrics.height as u32 > size {
                println!("[Font Error] Texture overflow for character '{character}' (codepoint #{c})");
                break;
            }

            // merge character bitmaps into the main atlas
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let src_idx = row * metrics.width + col;
                    let dest_idx = ((glyph_y + row as u32) * size + (glyph_x + col as u32)) as usize;
                    atlas_bitmap[dest_idx] = bitmap[src_idx];
                }
            }

            // create character glyph for renderer
            let plane_bounds = Vec4::new(
                metrics.bounds.xmin / scale, 
                metrics.bounds.ymin / scale,
                (metrics.bounds.xmin / scale) + (metrics.width as f32 / scale),
                (metrics.bounds.ymin / scale) + (metrics.height as f32 / scale),
            );

            let uv_bounds = Vec4::new(
                glyph_x as f32 / size as f32,
                glyph_y as f32 / size as f32,
                metrics.width as f32 / size as f32,
                metrics.height as f32 / size as f32,
            );

            glyphs.insert(character, CharacterGlyph {
                plane_bounds, 
                uv_bounds, 
                advance: metrics.advance_width / scale
            });

            current_x += metrics.width as u32 + (padding * 2)
        }

        // let atlas = FontUtils::generate_sdf(&atlas_bitmap, size, radius);

        (glyphs, atlas_bitmap)
    }

    // /// Generate an signed distance field (sdf) bitmap from an alpha mask bitmap.
    // /// 
    // /// * 'src' - the alpha mask bitmap as raw bytes
    // /// * 'width' - the width of the sdf bitmap
    // /// * 'height' - the height of the sdf bitmap
    // /// * 'radius' - the max search distance for the edge gradient in the sdf bitmap
    // fn generate_sdf(src: &[u8], size: u32, radius: f32) -> Vec<u8> {

    //     Vec::new()
    // }
}