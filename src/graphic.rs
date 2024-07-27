use alloc::collections::btree_map::BTreeMap;
use core::mem::swap;
use noto_sans_mono_bitmap::{get_raster, get_raster_width};
use noto_sans_mono_bitmap::{FontWeight, RasterHeight};

use super::cell::{Cell, Flags};
use super::color::Rgb888;

const FONT_WIDTH: usize = get_raster_width(FontWeight::Regular, FONT_HEIGHT);
const FONT_HEIGHT: RasterHeight = RasterHeight::Size20;

pub trait DrawTarget {
    fn size(&self) -> (usize, usize);
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb888);
}

type FgBgPair = (Rgb888, Rgb888);

pub struct TextOnGraphic<D: DrawTarget> {
    width: usize,
    height: usize,
    graphic: D,
    color_cache: BTreeMap<FgBgPair, ColorCache>,
}

impl<D: DrawTarget> TextOnGraphic<D> {
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }
}

impl<D: DrawTarget> TextOnGraphic<D> {
    pub fn new(graphic: D) -> Self {
        let (width, height) = graphic.size();
        Self {
            width: width / FONT_WIDTH,
            height: height / FONT_HEIGHT as usize,
            graphic,
            color_cache: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self, cell: Cell) {
        let (width, height) = self.graphic.size();
        for row in 0..height {
            for col in 0..width {
                self.graphic.draw_pixel(col, row, cell.background.to_rgb());
            }
        }
    }

    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        if row >= self.height || col >= self.width {
            return;
        }

        let mut foreground = cell.foreground.to_rgb();
        let mut background = cell.background.to_rgb();

        if cell.flags.contains(Flags::INVERSE) || cell.flags.contains(Flags::CURSOR_BLOCK) {
            swap(&mut foreground, &mut background);
        }

        if cell.flags.contains(Flags::HIDDEN) {
            foreground = background;
        }

        let font_weight = if cell.flags.contains(Flags::BOLD) {
            FontWeight::Bold
        } else {
            FontWeight::Regular
        };

        let char_raster = get_raster(cell.content, font_weight, FONT_HEIGHT)
            .unwrap_or_else(|| get_raster('\u{fffd}', font_weight, FONT_HEIGHT).unwrap());

        let (x_start, y_start) = (col * FONT_WIDTH, row * FONT_HEIGHT as usize);

        let color_cache = self
            .color_cache
            .entry((foreground, background))
            .or_insert_with(|| ColorCache::new(foreground, background));

        for (y, lines) in char_raster.raster().iter().enumerate() {
            for (x, &intensity) in lines.iter().enumerate() {
                let (r, g, b) = color_cache.colors[intensity as usize];
                self.graphic.draw_pixel(x_start + x, y_start + y, (r, g, b));
            }
        }

        if cell.flags.contains(Flags::CURSOR_BEAM) {
            for y in 0..FONT_HEIGHT as usize {
                let (r, g, b) = color_cache.colors[0xff as usize];
                self.graphic.draw_pixel(x_start, y_start + y, (r, g, b));
            }
        }

        if cell.flags.contains(Flags::UNDERLINE) || cell.flags.contains(Flags::CURSOR_UNDERLINE) {
            for x in 0..FONT_WIDTH {
                let (r, g, b) = color_cache.colors[0xff as usize];
                self.graphic
                    .draw_pixel(x_start + x, y_start + FONT_HEIGHT as usize - 1, (r, g, b));
            }
        }
    }
}

struct ColorCache {
    colors: [Rgb888; 256],
}

impl ColorCache {
    fn new(foreground: Rgb888, background: Rgb888) -> Self {
        let r_diff = foreground.0 as i32 - background.0 as i32;
        let g_diff = foreground.1 as i32 - background.1 as i32;
        let b_diff = foreground.2 as i32 - background.2 as i32;

        let mut colors = [(0u8, 0u8, 0u8); 256];

        for intensity in 0..256 {
            let weight = intensity as i32;
            let r = ((background.0 as i32 + (r_diff * weight / 0xff)).clamp(0, 255)) as u8;
            let g = ((background.1 as i32 + (g_diff * weight / 0xff)).clamp(0, 255)) as u8;
            let b = ((background.2 as i32 + (b_diff * weight / 0xff)).clamp(0, 255)) as u8;
            colors[intensity] = (r, g, b);
        }

        ColorCache { colors }
    }
}
