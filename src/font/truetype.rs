use ab_glyph::{Font, FontRef, PxScale, ScaleFont, VariableFont};
use alloc::{collections::BTreeMap, vec::Vec};

use super::{FontManager, FontWeight, Rasterized};

#[derive(Debug)]
pub struct TrueTypeFont {
    font: FontRef<'static>,
    raster_height: usize,
    raster_width: usize,
    font_size: PxScale,
    base_line_offset: f32,
    bitmap_cache: BTreeMap<(char, FontWeight), Vec<Vec<u8>>>,
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
            raster_height,
            raster_width,
            font_size,
            base_line_offset,
            bitmap_cache: BTreeMap::new(),
        }
    }
}

impl FontManager for TrueTypeFont {
    fn size(&self) -> (usize, usize) {
        (self.raster_width, self.raster_height)
    }

    fn rasterize(&mut self, content: char, weight: FontWeight) -> Rasterized {
        if let Some(bitmap) = self.bitmap_cache.get(&(content, weight.clone())) {
            return Rasterized::Owned(bitmap.clone());
        }

        let font_weight = match weight {
            FontWeight::Regular => 400.0,
            FontWeight::Bold => 700.0,
        };

        self.font.set_variation(b"wght", font_weight);

        let glyph = self.font.glyph_id(content).with_scale(self.font_size);
        let mut letter_bitmap = vec![vec![0u8; self.raster_width]; self.raster_height];

        if let Some(bitmap) = self.font.outline_glyph(glyph) {
            let px_bounds = bitmap.px_bounds();

            let x_offset = px_bounds.min.x as isize;
            let y_offset = (self.base_line_offset + px_bounds.min.y) as isize;

            bitmap.draw(|x, y, c| {
                let x = x_offset + x as isize;
                let y = y_offset + y as isize;

                if (x >= 0 && x < self.raster_width as isize)
                    && (y >= 0 && y < self.raster_height as isize)
                {
                    letter_bitmap[y as usize][x as usize] = (c * 255.0) as u8;
                }
            });
        }

        self.bitmap_cache.insert((content, weight), letter_bitmap.clone());

        Rasterized::Owned(letter_bitmap)
    }
}
