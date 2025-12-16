use ab_glyph::{Font, FontRef, PxScale};
use ab_glyph::{ScaleFont, VariableFont};
use alloc::vec::Vec;
use core::num::NonZeroUsize;
use lru::LruCache;

use super::{ContentInfo, FontManager, Rasterized};

pub struct TrueTypeFont {
    font: FontRef<'static>,
    italic_font: Option<FontRef<'static>>,
    raster_height: usize,
    raster_width: usize,
    font_size: PxScale,
    base_line_offset: f32,
    bitmap_cache: LruCache<ContentInfo, Vec<Vec<u8>>>,
}

impl TrueTypeFont {
    pub fn new(font_size: f32, font_bytes: &'static [u8]) -> Self {
        let font = FontRef::try_from_slice(font_bytes).unwrap();
        let font_size = font.pt_to_px_scale(font_size).unwrap();
        let scaled_font = font.as_scaled(font_size);

        let line_height = scaled_font.height();
        let base_line_offset = scaled_font.ascent();

        Self {
            font,
            italic_font: None,
            raster_height: line_height as usize,
            raster_width: (line_height / 2.0) as usize,
            font_size,
            base_line_offset,
            bitmap_cache: LruCache::new(NonZeroUsize::new(512).unwrap()),
        }
    }
    
    pub fn with_cache_size(mut self, size: usize) -> Self {
        assert!(size > 0, "Cache size must be greater than 0");
        self.bitmap_cache.resize(NonZeroUsize::new(size).unwrap());
        self
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

    fn rasterize(&mut self, info: ContentInfo) -> Rasterized<'_> {
        Rasterized::Vec(self.bitmap_cache.get_or_insert(info.clone(), || {
            let select_font = self
                .italic_font
                .as_mut()
                .filter(|_| info.italic)
                .unwrap_or(&mut self.font);

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

                    if (0..actual_width as isize).contains(&x)
                        && (0..self.raster_height as isize).contains(&y)
                    {
                        letter_bitmap[y as usize][x as usize] = (c * 255.0) as u8;
                    }
                });
            }

            letter_bitmap
        }))
    }
}
