use alloc::collections::btree_map::BTreeMap;
use core::mem::swap;
use noto_sans_mono_bitmap::{get_raster, get_raster_width};
use noto_sans_mono_bitmap::{FontWeight, RasterHeight};

use super::cell::{Cell, Flags};

const FONT_WIDTH: usize = get_raster_width(FontWeight::Regular, FONT_HEIGHT);
const FONT_HEIGHT: RasterHeight = RasterHeight::Size20;

pub trait DrawTarget {
    fn size(&self) -> (usize, usize);
    fn draw_pixel(&mut self, x: usize, y: usize, color: (u8, u8, u8));
}

type BgFgPair = ((u8, u8, u8), (u8, u8, u8));

pub struct TextOnGraphic<D: DrawTarget> {
    width: usize,
    height: usize,
    graphic: D,
    color_cache: BTreeMap<BgFgPair, ColorCache>,
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
            .entry((background, foreground))
            .or_insert_with(|| ColorCache::new(foreground, background));

        let mut draw_pixel = |x: usize, y: usize, intensity: u8| {
            let r = color_cache.r_cache[intensity as usize];
            let g = color_cache.g_cache[intensity as usize];
            let b = color_cache.b_cache[intensity as usize];

            self.graphic.draw_pixel(x_start + x, y_start + y, (r, g, b));
        };

        for (y, lines) in char_raster.raster().iter().enumerate() {
            for (x, intensity) in lines.iter().enumerate() {
                draw_pixel(x, y, *intensity);
            }
        }

        if cell.flags.contains(Flags::CURSOR_BEAM) {
            for y in 0..FONT_HEIGHT as usize {
                draw_pixel(0, y, 0xff);
            }
        }

        if cell.flags.contains(Flags::UNDERLINE) || cell.flags.contains(Flags::CURSOR_UNDERLINE) {
            for x in 0..FONT_WIDTH {
                draw_pixel(x, FONT_HEIGHT as usize - 1, 0xff);
            }
        }
    }
}

struct ColorCache {
    r_cache: [u8; 256],
    g_cache: [u8; 256],
    b_cache: [u8; 256],
}

impl ColorCache {
    fn new(foreground: (u8, u8, u8), background: (u8, u8, u8)) -> Self {
        let r_diff = foreground.0 as i32 - background.0 as i32;
        let g_diff = foreground.1 as i32 - background.1 as i32;
        let b_diff = foreground.2 as i32 - background.2 as i32;

        let mut r_cache = [0u8; 256];
        let mut g_cache = [0u8; 256];
        let mut b_cache = [0u8; 256];

        for intensity in 0..256 {
            let weight = intensity as i32;
            r_cache[intensity] =
                ((background.0 as i32 + (r_diff * weight / 0xff)).clamp(0, 255)) as u8;
            g_cache[intensity] =
                ((background.1 as i32 + (g_diff * weight / 0xff)).clamp(0, 255)) as u8;
            b_cache[intensity] =
                ((background.2 as i32 + (b_diff * weight / 0xff)).clamp(0, 255)) as u8;
        }

        ColorCache {
            r_cache,
            g_cache,
            b_cache,
        }
    }
}
