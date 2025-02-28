use noto_sans_mono_bitmap::{FontWeight, RasterHeight};
use noto_sans_mono_bitmap::{get_raster, get_raster_width};

use super::{ContentInfo, FontManager, Rasterized};

const FONT_WIDTH: usize = get_raster_width(FontWeight::Regular, FONT_HEIGHT);
const FONT_HEIGHT: RasterHeight = RasterHeight::Size20;

pub struct BitmapFont;

impl FontManager for BitmapFont {
    fn size(&self) -> (usize, usize) {
        (FONT_WIDTH, FONT_HEIGHT as usize)
    }

    fn rasterize(&mut self, info: ContentInfo) -> Rasterized {
        let font_weight = if info.bold {
            FontWeight::Bold
        } else {
            FontWeight::Regular
        };

        let char_raster = get_raster(info.content, font_weight, FONT_HEIGHT)
            .unwrap_or(get_raster('\u{fffd}', font_weight, FONT_HEIGHT).unwrap());

        Rasterized::Slice(char_raster.raster())
    }
}
