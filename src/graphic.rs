use alloc::collections::btree_map::BTreeMap;
use core::mem::swap;

use crate::cell::{Cell, Flags};
use crate::color::Rgb888;
use crate::config::CONFIG;
use crate::font::{ContentInfo, Rasterized};

pub trait DrawTarget {
    fn size(&self) -> (usize, usize);
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb888);
}

type FgBgPair = (Rgb888, Rgb888);

pub struct TextOnGraphic<D: DrawTarget> {
    graphic: D,
    color_cache: BTreeMap<FgBgPair, ColorCache>,
}

impl<D: DrawTarget> TextOnGraphic<D> {
    #[inline]
    pub fn width(&self) -> usize {
        self.graphic.size().0
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.graphic.size().1
    }
}

impl<D: DrawTarget> TextOnGraphic<D> {
    pub fn new(graphic: D) -> Self {
        Self {
            graphic,
            color_cache: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self, cell: Cell) {
        let (width, height) = self.graphic.size();
        for y in 0..height {
            for x in 0..width {
                self.graphic.draw_pixel(x, y, cell.background.to_rgb());
            }
        }
    }

    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        let mut foreground = cell.foreground.to_rgb();
        let mut background = cell.background.to_rgb();

        if cell.flags.contains(Flags::INVERSE) || cell.flags.contains(Flags::CURSOR_BLOCK) {
            swap(&mut foreground, &mut background);
        }

        if cell.flags.contains(Flags::HIDDEN) {
            foreground = background;
        }

        let color_cache = self
            .color_cache
            .entry((foreground, background))
            .or_insert_with(|| ColorCache::new(foreground, background));

        if let Some(font_manager) = CONFIG.lock().font_manager.as_mut() {
            let (font_width, font_height) = font_manager.size();
            let (x_start, y_start) = (col * font_width, row * font_height);

            let content_info = ContentInfo::new(
                cell.content,
                cell.flags.contains(Flags::BOLD),
                cell.flags.contains(Flags::ITALIC),
            );

            macro_rules! draw_raster {
                ($raster:ident) => {
                    for (y, lines) in $raster.iter().enumerate() {
                        for (x, &intensity) in lines.iter().enumerate() {
                            let (r, g, b) = color_cache.colors[intensity as usize];
                            self.graphic.draw_pixel(x_start + x, y_start + y, (r, g, b));
                        }
                    }
                };
            }

            match font_manager.rasterize(content_info) {
                Rasterized::Borrowed(raster) => draw_raster!(raster),
                Rasterized::Owned(raster) => draw_raster!(raster),
            }

            if cell.flags.contains(Flags::CURSOR_BEAM) {
                for y in 0..font_height as usize {
                    let (r, g, b) = color_cache.colors[0xff as usize];
                    self.graphic.draw_pixel(x_start, y_start + y, (r, g, b));
                }
            }

            if cell.flags.contains(Flags::UNDERLINE) || cell.flags.contains(Flags::CURSOR_UNDERLINE)
            {
                for x in 0..font_width {
                    let (r, g, b) = color_cache.colors[0xff as usize];
                    self.graphic.draw_pixel(
                        x_start + x,
                        y_start + font_height as usize - 1,
                        (r, g, b),
                    );
                }
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
