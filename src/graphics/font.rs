use std::{collections::HashMap, fs::File, io::Read, sync::Arc};
use rayon::prelude::*;
use glam::Vec4;

use crate::graphics::{geometry::Geometry, handler::{BuilderType, ResourceBuilder}, instance::InstanceGroup, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, primitive::{Primitive, RenderInfo}, shape_factory::Shape2D, texture::{TextureBuilder, TextureContext}, transform::Transform, vertex::{TransformAttribute, Vec2Attribute, Vec3Attribute, Vec4Attribute, attr}, wpgu_context::{ResourceID, ResourceScope, ResourceType}};

/// Data struct describing the properties of a font's texture atlas
#[derive(Clone, Debug)]
pub struct FontDescriptor {
    pub path: String,
    pub atlas_size: u32,
    pub scale: f32,
    pub sdf_radius: f32,
}

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

/// Handle for a font's atlas texture and how to read it.
#[derive(Clone, Debug)]
pub struct FontAsset {
    /// The handle to the gpu texture font atlas
    pub atlas: Arc<wgpu::TextureView>,
    /// The font glyph information used for rendering text
    pub glyphs: HashMap<char, CharacterGlyph>,
    /// The spacing between lines of text rendered with this font
    pub line_height: f32,
}

impl FontAsset {
    /// Calculate the size of text rendered with this font needed to fit within a certain width without scaling issues.
    /// 
    /// * 'text' - the text to calculate the size of
    /// * 'width' - the width of the final rendered text in NDC.
    pub fn calc_size(&self, text: &str, width: f32) -> f32 {
        let lines: Vec<&str> = text.split('\n').collect();

        let mut unscaled_width: f32 = 0.0;
        for line in &lines {
            let mut curr_line_width = 0.0;

            for character in line.chars() {
                if let Some(glyph) = self.glyphs.get(&character) {
                    curr_line_width += glyph.advance;
                }
            }

            if curr_line_width > unscaled_width {
                unscaled_width = curr_line_width;
            }
        }

        width / unscaled_width
    }
}

/// Used to render text from a given ttf font file
/// 
/// Serves as a handle to the matching font texture atlas and glyph table.
#[derive(Clone, Debug)]
pub struct Font {
    /// the unique id of this font
    id: ResourceID,
    /// the font description used to create the associated font asset
    desc: FontDescriptor,
}

impl Font {
    pub fn new(path: &str) -> Self {
        let id = ResourceID {
            key: path.to_string(),
            scope: ResourceScope::Global,
            r_type: ResourceType::Font
        };

        Self {
            desc: FontDescriptor { 
                path: path.to_string(),
                atlas_size: 1024, 
                scale: 96.0, 
                sdf_radius: 12.0 
            },
            id
        }
    }

    /// Create a new primitive to represent this font during rendering
    /// 
    /// * 'instance_cap' - the total number of characters this font is allowed to render
    pub fn create_primitive(&self, instance_cap: usize) -> Primitive {
        let geometry = Geometry::new(Shape2D::new().square())
            .with_attribute(Vec3Attribute(attr::POSITION))
            .with_attribute(Vec2Attribute(attr::UV_COORDS));

        let instances = InstanceGroup::new(0, instance_cap) // allow space for 1000 characters
            .with_label(&format!("font::{}", self.desc.path))
            .with_attribute(TransformAttribute, Vec::<Transform>::with_capacity(instance_cap))
            .with_attribute(Vec4Attribute(attr::TEXT_COLOR), Vec::<Vec4>::with_capacity(instance_cap))
            .with_attribute(Vec4Attribute(attr::OUTLINE_COLOR), Vec::<Vec4>::with_capacity(instance_cap))
            .with_attribute(Vec4Attribute(attr::UV_BOUNDS), Vec::<Vec4>::with_capacity(instance_cap));

        let label = format!("font::{}", self.desc.path);
        
        Primitive::from_group(
            &label, 
            geometry, 
            MaterialPreset::Font(self.id.clone()).with_label(&label), 
            instances, 
            RenderInfo {
                shader_path: ShaderSpecPreset::Font.path(),
                pipeline: RenderPipeline::Font.get()
            },
        )
    }

    /// get the font builder associated with this font
    pub fn get_builder(&self) -> FontBuilder {
        FontBuilder::new(self.desc.clone())
    }

    /// get the id associated with this font
    pub fn get_id(&self) -> ResourceID {
        self.id.clone()
    }

    /// get a copy of the descriptor for this font
    pub fn get_descriptor(&self) -> FontDescriptor {
        self.desc.clone()
    }
}

/// Creates font atlases for use in rendering fonts
#[derive(Clone, Debug)]
pub struct FontBuilder {
    desc: FontDescriptor,
}

impl FontBuilder {
    pub fn new(desc: FontDescriptor) -> Self {
        Self { desc }
    }
}

impl ResourceBuilder for FontBuilder {
    type Output = Arc<FontAsset>;
    type Context = TextureContext;

    fn builder_type(&self) -> super::handler::BuilderType { BuilderType::Blocking }

    fn build(&self, context: Arc<Self::Context>) -> Result<Self::Output, String> {
        let mut ttf_file = File::open(&self.desc.path).map_err(|e| e.to_string())?;
        let mut font_data = Vec::new();
        ttf_file.read_to_end(&mut font_data).map_err(|e| e.to_string())?;

        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        let line_metrics = font.horizontal_line_metrics(self.desc.scale)
            .ok_or("Failed to read font line metrics.")?;
        let line_height = line_metrics.new_line_size / self.desc.scale;

        let (glyphs, atlas_bytes) = FontUtils::gen_font_atlas(
            font, 
            self.desc.atlas_size,
            self.desc.scale,
            self.desc.sdf_radius
        );

        let atlas = TextureBuilder::new()
            .with_label(&self.desc.path)
            .with_data(self.desc.atlas_size, self.desc.atlas_size, atlas_bytes)
            .with_format(wgpu::TextureFormat::R8Unorm)
            .build(context)?;

        let font_asset = FontAsset { atlas, glyphs, line_height };

        return Ok(Arc::new(font_asset));
    }
}

/// Helper functions for generating font atlases
pub struct FontUtils;
impl FontUtils {
    /// Generate an signed distance field (sdf) font atlas by rasterizing a ttf font
    /// 
    /// * 'font' - the font containing the glyph metrics
    /// * 'size' - the size of the bitmap atlas in pixels
    /// * 'scale' - the size to rasterize the characters with, in pixels per em unit
    /// * 'radius' - spacing between bitmap characters
    pub fn gen_font_atlas(font: fontdue::Font, size: u32, scale: f32, radius: f32) -> (HashMap<char, CharacterGlyph>, Vec<u8>) {
        let padding = radius as u32;
        let spacing: u32 = 2; // spacing between glyphs in the atlas to prevent bleeding

        let characters: Vec<u8> = (32u8..127).collect();
        let sdf_glyphs: Vec<SdfGlyph> = characters
            .into_par_iter()
            .map(|u_char| {
                let ch = u_char as char;

                let (metrics, bitmap) = font.rasterize(ch, scale);

                let padded_width = metrics.width as u32 + (padding * 2);
                let padded_height = metrics.height as u32 + (padding * 2);

                // Pad the raw character bitmap and do sdf processing
                let mut padded_bitmap = vec![0u8; (padded_width * padded_height) as usize];
                for r in 0..metrics.height {
                    for c in 0..metrics.width {
                        let src_idx = r * metrics.width + c;
                        let pad_idx = (r + padding as usize) * padded_width as usize + c + padding as usize;

                        padded_bitmap[pad_idx] = bitmap[src_idx];
                    }
                }

                let sdf_bitmap = FontUtils::generate_sdf(&padded_bitmap, padded_width, padded_height, radius);

                SdfGlyph {
                    ch, 
                    metrics,
                    bitmap: sdf_bitmap,
                    width: padded_width,
                    height: padded_height
                }
            })
            .collect();

        FontUtils::blit_glyphs(sdf_glyphs, spacing, size, radius)
    }

    /// blit character glyphs into a font atlas, and convert metrics into glyph bounds
    fn blit_glyphs(glyphs: Vec<SdfGlyph>, spacing: u32, size: u32, radius: f32) -> (HashMap<char, CharacterGlyph>, Vec<u8>){
        let mut atlas_bitmap = vec![0u8; (size * size) as usize];
        let mut glyph_table = HashMap::new();

        let mut current_x: u32 = 0;
        let mut current_y: u32 = 0;
        let mut max_row_height: u32 = 0;

        for glyph in glyphs {
            // wrap to next row and add 2px spacing between glyphs
            if current_x + glyph.width + spacing > size {
                current_x = 0;
                current_y += max_row_height + spacing;
                max_row_height = 0;
            }
            max_row_height = max_row_height.max(glyph.height);

            // blit character sdf bitmap into atlas
            for r in 0..glyph.height {
                for c in 0..glyph.width {
                    let src_idx = (r * glyph.width + c) as usize;

                    let atlas_x = current_x + c;
                    let atlas_y = current_y + r;
                    let atlas_idx = (atlas_y * size + atlas_x) as usize;

                    atlas_bitmap[atlas_idx] = glyph.bitmap[src_idx];
                }
            }

            // insert character glyph metrics into glyph table
            let uv_bounds = Vec4::new(
                current_x as f32 / size as f32,
                current_y as f32 / size as f32,
                glyph.width as f32 / size as f32,
                glyph.height as f32 / size as f32,
            );

            let plane_bounds = Vec4::new(
                glyph.metrics.bounds.xmin as f32 - radius,
                glyph.metrics.bounds.ymin as f32 - radius,
                glyph.metrics.width as f32 + (radius * 2.0),
                glyph.metrics.height as f32 + (radius * 2.0)
            );

            glyph_table.insert(glyph.ch, CharacterGlyph {
                plane_bounds,
                uv_bounds,
                advance: glyph.metrics.advance_width
            });

            // advance x coord for next character
            current_x += glyph.width + spacing;
        }

        (glyph_table, atlas_bitmap)
    }

    /// Generate a signed distance field (sdf) bitmap from an alpha mask bitmap.
    /// 
    /// * 'src' - the alpha mask bitmap as raw bytes
    /// * 'width' - the width of the sdf bitmap
    /// * 'height' - the height of the sdf bitmap
    /// * 'radius' - the max search distance for the edge gradient in the sdf bitmap
    fn generate_sdf(src: &[u8], width: u32, height: u32, radius: f32) -> Vec<u8> {
        let width = width as i32;
        let height = height as i32;
        let total_pixels = (width * height) as usize;

        // We maintain two separate vector grids: one for the interior of the text, one for the exterior.
        // This allows us to calculate an accurate "Signed" distance field from both sides of the edge.
        let mut grid_inside = vec![Point::infinity(); total_pixels];
        let mut grid_outside = vec![Point::infinity(); total_pixels];

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let alpha = src[idx] as f32 / 255.0;
                
                if alpha > 0.5 { 
                    grid_inside[idx] = Point { dx: 0.0, dy: 0.0 };
                } else {
                    grid_outside[idx] = Point { dx: 0.0, dy: 0.0 };
                }
            }
        }

        // Helper macro/closure to perform sequential neighbor evaluation
        let compare = |grid: &mut Vec<Point>, x: i32, y: i32, nx: i32, ny: i32| {
            if nx >= 0 && nx < width && ny >= 0 && ny < height {
                let curr_idx = (y * width + x) as usize;
                let neigh_idx = (ny * width + nx) as usize;
                
                let n_pt = unsafe { *grid.get_unchecked(neigh_idx) };
                // Calculate relative offset step to this neighbor
                let new_pt = Point {
                    dx: n_pt.dx + ((nx - x) as f32),
                    dy: n_pt.dy + ((ny - y) as f32),
                };

                if new_pt.length_sq() < grid[curr_idx].length_sq() {
                    grid[curr_idx] = new_pt;
                }
            }
        };

        // 2. THE FORWARD SWEEP (Top-to-Bottom, Left-to-Right)
        for y in 0..height {
            for x in 0..width {
                // Check Left, Top-Left, Top, Top-Right
                compare(&mut grid_inside, x, y, x - 1, y);
                compare(&mut grid_inside, x, y, x - 1, y - 1);
                compare(&mut grid_inside, x, y, x,     y - 1);
                compare(&mut grid_inside, x, y, x + 1, y - 1);

                compare(&mut grid_outside, x, y, x - 1, y);
                compare(&mut grid_outside, x, y, x - 1, y - 1);
                compare(&mut grid_outside, x, y, x,     y - 1);
                compare(&mut grid_outside, x, y, x + 1, y - 1);
            }
        }

        // 3. THE BACKWARD SWEEP (Bottom-to-Top, Right-to-Left)
        for y in (0..height).rev() {
            for x in (0..width).rev() {
                // Check Right, Bottom-Right, Bottom, Bottom-Left
                compare(&mut grid_inside, x, y, x + 1, y);
                compare(&mut grid_inside, x, y, x + 1, y + 1);
                compare(&mut grid_inside, x, y, x,     y + 1);
                compare(&mut grid_inside, x, y, x - 1, y + 1);

                compare(&mut grid_outside, x, y, x + 1, y);
                compare(&mut grid_outside, x, y, x + 1, y + 1);
                compare(&mut grid_outside, x, y, x,     y + 1);
                compare(&mut grid_outside, x, y, x - 1, y + 1);
            }
        }

        let inv_radius = 1.0 / radius;
        let dest: Vec<u8> = grid_inside
            .into_iter()
            .enumerate()
            .map(|(i, point)| {
                let is_inside = src[i] > 127;

                // Calculate absolute true Euclidean distance from our displacement vectors
                let dist = if is_inside {
                    grid_outside[i].length_sq().sqrt()
                } else {
                    point.length_sq().sqrt()
                };

                // Clamp distance to our max radius boundary, and normalize 0.0 to 1.0
                let normalized = dist.min(radius) * inv_radius * 0.5;

                // Apply our center-biased threshold (0.5 is the exact edge)
                let final_sdf = if is_inside {
                    0.5 + normalized
                } else {
                    0.5 - normalized
                };

                (final_sdf * 255.0).round() as u8
            })
            .collect();

        dest
    }
}

#[derive(Clone, Copy, Debug)]
struct Point {
    dx: f32,
    dy: f32,
}

impl Point {
    // An infinite/maximum distance marker for initialization
    fn infinity() -> Self {
        Self { dx: 9999.0, dy: 9999.0 }
    }

    fn length_sq(&self) -> f32 {
        self.dx * self.dx + self.dy * self.dy
    }
}

/// temporary struct generated by rayon threads.
pub struct SdfGlyph {
    ch: char,
    metrics: fontdue::Metrics,
    bitmap: Vec<u8>,
    width: u32,
    height: u32,
}