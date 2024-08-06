use noto_sans_mono_bitmap::{get_raster, get_raster_width};
use noto_sans_mono_bitmap::{FontWeight as BitmapFontWeight, RasterHeight};

use super::{FontManager, FontWeight, Rasterized};

const FONT_WIDTH: usize = get_raster_width(BitmapFontWeight::Regular, FONT_HEIGHT);
const FONT_HEIGHT: RasterHeight = RasterHeight::Size20;

pub struct BitmapFont;

impl FontManager for BitmapFont {
    fn size(&self) -> (usize, usize) {
        (FONT_WIDTH, FONT_HEIGHT as usize)
    }

    fn rasterize(&mut self, content: char, _weight: FontWeight) -> Rasterized {
        let font_weight = match _weight {
            FontWeight::Regular => BitmapFontWeight::Regular,
            FontWeight::Bold => BitmapFontWeight::Bold,
        };

        let char_raster = get_raster(content, font_weight, FONT_HEIGHT)
            .unwrap_or_else(|| get_raster('\u{fffd}', font_weight, FONT_HEIGHT).unwrap());

        Rasterized::Borrowed(char_raster.raster())
    }
}
