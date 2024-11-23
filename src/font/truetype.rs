use ab_glyph::{Font, FontRef, PxScale, ScaleFont, VariableFont};
use alloc::{collections::BTreeMap, vec::Vec};

use super::{ContentInfo, FontManager, Rasterized};

pub struct TrueTypeFont {
    font: FontRef<'static>,
    italic_font: Option<FontRef<'static>>,
    raster_height: usize,
    raster_width: usize,
    font_size: PxScale,
    base_line_offset: f32,
    bitmap_cache: BTreeMap<ContentInfo, Vec<Vec<u8>>>,
}

impl TrueTypeFont {
    pub fn new(font_size: f32, font_bytes: &'static [u8]) -> Self {
        let font = FontRef::try_from_slice(font_bytes).unwrap();
        let font_size = font.pt_to_px_scale(font_size).unwrap();

        let line_height = font.as_scaled(font_size).height();
        let base_line_offset = font.as_scaled(font_size).ascent();

        let raster_height = line_height as usize;
        let raster_width = (line_height / 2.0) as usize;

        Self {
            font,
            italic_font: None,
            raster_height,
            raster_width,
            font_size,
            base_line_offset,
            bitmap_cache: BTreeMap::new(),
        }
    }

    pub fn with_italic_font(mut self, italic_font: &'static [u8]) -> Self {
        self.italic_font = Some(FontRef::try_from_slice(italic_font).unwrap());
        self
    }
}

impl FontManager for TrueTypeFont {
    fn size(&self) -> (usize, usize) {
        (self.raster_width, self.raster_height)
    }

    fn rasterize(&mut self, info: ContentInfo) -> Rasterized {
        Rasterized::Vec(self.bitmap_cache.entry(info.clone()).or_insert_with(|| {
            let select_font = if info.italic {
                self.italic_font.as_mut().unwrap_or(&mut self.font)
            } else {
                &mut self.font
            };

            let font_weight = if info.bold { 700.0 } else { 400.0 };
            select_font.set_variation(b"wght", font_weight);

            let glyph_id = select_font.glyph_id(info.content);
            let glyph = glyph_id.with_scale(self.font_size);

            let actual_width = self.raster_width * if info.wide { 2 } else { 1 };
            let mut letter_bitmap = vec![vec![0u8; actual_width]; self.raster_height];

            if let Some(bitmap) = select_font.outline_glyph(glyph) {
                let px_bounds = bitmap.px_bounds();

                let x_offset = px_bounds.min.x as isize;
                let y_offset = (self.base_line_offset + px_bounds.min.y) as isize;

                bitmap.draw(|x, y, c| {
                    let x = x_offset + x as isize;
                    let y = y_offset + y as isize;

                    if (x >= 0 && x < actual_width as isize)
                        && (y >= 0 && y < self.raster_height as isize)
                    {
                        letter_bitmap[y as usize][x as usize] = (c * 255.0) as u8;
                    }
                });
            }

            letter_bitmap
        }))
    }
}
