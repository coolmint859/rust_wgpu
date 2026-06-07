use std::{collections::HashMap, fs::File, io::Read, sync::Arc};

use glam::Vec4;

use crate::graphics::{geometry::{Geometry, PositionAttribute, UVAttribute}, handler::ResourceBuilder, instance::{InstanceGroup, TintAttribute, TransformAttribute, UVBoundsAttribute}, presets::{MaterialPreset, RenderPipeline, ShaderSpecPreset}, primitive::{Primitive, RenderInfo}, shape_factory::Shape2D, texture::{TextureBuilder, TextureContext}, transform::Transform, wpgu_context::{ResourceID, ResourceScope, ResourceType}};

pub const TEXT_COLOR: &str = "text_color";
pub const OUTLINE_COLOR: &str = "outline_color";

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
            .with_attribute(PositionAttribute)
            .with_attribute(UVAttribute)
            .with_attribute(UVBoundsAttribute);

        let instances = InstanceGroup::new(0, instance_cap) // allow space for 1000 characters
            .with_label(&format!("font::{}", self.desc.path))
            .with_attribute(TransformAttribute, Vec::<Transform>::with_capacity(instance_cap))
            .with_attribute(TintAttribute(TEXT_COLOR, 10), Vec::<Vec4>::with_capacity(instance_cap))
            .with_attribute(TintAttribute(OUTLINE_COLOR, 11), Vec::<Vec4>::with_capacity(instance_cap))
            .with_attribute(UVBoundsAttribute, Vec::<Vec4>::with_capacity(instance_cap));

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

    fn build(&self, context: Arc<Self::Context>) -> Result<Self::Output, String> {
        let mut ttf_file = File::open(&self.desc.path).map_err(|e| e.to_string())?;
        let mut font_data = Vec::new();
        ttf_file.read_to_end(&mut font_data).map_err(|e| e.to_string())?;

        let font = fontdue::Font::from_bytes(font_data, fontdue::FontSettings::default())?;
        let line_metrics = font.horizontal_line_metrics(self.desc.scale)
            .ok_or("Failed to read font line metrics.")?;
        let line_height = line_metrics.new_line_size / self.desc.scale;

        let (glyphs, atlas_bytes) = FontUtils::gen_sdf_font(
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

            let rad_plane_unit = radius / scale;

            // create character glyph for renderer
            let plane_bounds = Vec4::new(
                (metrics.bounds.xmin / scale) - rad_plane_unit, 
                (metrics.bounds.ymin / scale) - rad_plane_unit,
                (metrics.bounds.xmin / scale) + (metrics.width as f32 / scale) + rad_plane_unit,
                (metrics.bounds.ymin / scale) + (metrics.height as f32 / scale) + rad_plane_unit,
            );

            let uv_bounds = Vec4::new(
                (glyph_x as f32 - radius) / size as f32,
                (glyph_y as f32 - radius) / size as f32,
                (metrics.width as f32 + (radius * 2.0)) / size as f32,
                (metrics.height as f32 + (radius * 2.0)) / size as f32,
            );

            glyphs.insert(character, CharacterGlyph {
                plane_bounds, 
                uv_bounds, 
                advance: metrics.advance_width / scale
            });

            current_x += metrics.width as u32 + (padding * 2)
        }

        let atlas = FontUtils::generate_sdf(&atlas_bitmap, size, radius);

        (glyphs, atlas)
    }

    /// Generate a signed distance field (sdf) bitmap from an alpha mask bitmap.
    /// 
    /// * 'src' - the alpha mask bitmap as raw bytes
    /// * 'width' - the width of the sdf bitmap
    /// * 'height' - the height of the sdf bitmap
    /// * 'radius' - the max search distance for the edge gradient in the sdf bitmap
    fn generate_sdf(src: &[u8], size: u32, radius: f32) -> Vec<u8> {
        let width = size as i32;
        let height = size as i32;
        let total_pixels = (size * size) as usize;

        // We maintain two separate vector grids: one for the interior of the text, one for the exterior.
        // This allows us to calculate an accurate "Signed" distance field from both sides of the edge.
        let mut grid_inside = vec![Point::infinity(); total_pixels];
        let mut grid_outside = vec![Point::infinity(); total_pixels];

        // 1. INITIALIZATION PASS
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                // let is_inside = src[idx] > 127;

                // if is_inside {
                //     grid_inside[idx] = Point { dx: 0, dy: 0 };
                // } else {
                //     grid_outside[idx] = Point { dx: 0, dy: 0 };
                // }

                let alpha = src[idx] as f32 / 255.0;
                if alpha > 0.0 && alpha < 1.0 {
                    let offset = 0.5 - alpha;
                    grid_inside[idx] = Point { dx: offset, dy: offset };
                    grid_outside[idx] = Point { dx: -offset, dy: -offset }
                }
            }
        }

        // Helper macro/closure to perform sequential neighbor evaluation
        let compare = |grid: &mut Vec<Point>, x: i32, y: i32, nx: i32, ny: i32| {
            if nx >= 0 && nx < width && ny >= 0 && ny < height {
                let curr_idx = (y * width + x) as usize;
                let neigh_idx = (ny * width + nx) as usize;
                
                let n_pt = grid[neigh_idx];
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

        // 4. FINAL DISTANCE MAPPING PASS
        let mut dest = vec![0u8; total_pixels];
        for idx in 0..total_pixels {
            let is_inside = src[idx] > 127;

            // Calculate absolute true Euclidean distance from our displacement vectors
            let dist = if is_inside {
                (grid_outside[idx].length_sq() as f32).sqrt()
            } else {
                (grid_inside[idx].length_sq() as f32).sqrt()
            };

            // Clamp distance to our max radius boundary, and normalize 0.0 to 1.0
            let normalized = (dist.min(radius) / radius) * 0.5;

            // Apply our center-biased threshold (0.5 is the exact edge)
            let final_sdf = if is_inside {
                0.5 + normalized
            } else {
                0.5 - normalized
            };

            dest[idx] = (final_sdf * 255.0).clamp(0.0, 255.0) as u8;
        }

        dest
    }

    // fn compare_point(&mut points: Vec<Point>, x: i32, y: i32, nx: i32, ny: i32) {
    //     if nx >= 0 && nx < width && ny >= 0 && ny < height {
    //         let curr_idx = (y * width + x) as usize;
    //         let neigh_idx = (ny * width + nx) as usize;
            
    //         let n_pt = grid[neigh_idx];
    //         // Calculate relative offset step to this neighbor
    //         let new_pt = Point {
    //             dx: n_pt.dx + (nx - x),
    //             dy: n_pt.dy + (ny - y),
    //         };

    //         if new_pt.length_sq() < grid[curr_idx].length_sq() {
    //             grid[curr_idx] = new_pt;
    //         }
    //     }
    // }
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