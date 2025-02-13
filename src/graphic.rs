use alloc::collections::btree_map::BTreeMap;
use core::mem::swap;
use derive_more::{Deref, DerefMut};

use crate::cell::{Cell, Flags};
use crate::color::Rgb;
use crate::config::CONFIG;
use crate::font::{ContentInfo, Rasterized};

pub trait DrawTarget {
    fn size(&self) -> (usize, usize);
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb);
}

#[derive(Deref, DerefMut)]
pub struct Graphic<D: DrawTarget> {
    #[deref]
    #[deref_mut]
    graphic: D,
    color_cache: BTreeMap<(Rgb, Rgb), ColorCache>,
}

impl<D: DrawTarget> Graphic<D> {
    pub fn new(graphic: D) -> Self {
        Self {
            graphic,
            color_cache: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self, cell: Cell) {
        let color = cell.background.to_rgb();

        for y in 0..self.graphic.size().1 {
            for x in 0..self.graphic.size().0 {
                self.graphic.draw_pixel(x, y, color);
            }
        }
    }

    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        if cell.placeholder {
            return;
        }

        let mut foreground = cell.foreground.to_rgb();
        let mut background = cell.background.to_rgb();

        if cell.flags.intersects(Flags::INVERSE | Flags::CURSOR_BLOCK) {
            swap(&mut foreground, &mut background);
        }

        if cell.flags.contains(Flags::HIDDEN) {
            foreground = background;
        }

        if let Some(font_manager) = CONFIG.font_manager.lock().as_mut() {
            let (font_width, font_height) = font_manager.size();
            let (x_start, y_start) = (col * font_width, row * font_height);

            let color_cache = self
                .color_cache
                .entry((foreground, background))
                .or_insert_with(|| ColorCache::new(foreground, background));

            let content_info = ContentInfo {
                content: cell.content,
                bold: cell.flags.contains(Flags::BOLD),
                italic: cell.flags.contains(Flags::ITALIC),
                wide: cell.wide,
            };

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
                Rasterized::Slice(raster) => draw_raster!(raster),
                Rasterized::Vec(raster) => draw_raster!(raster),
                Rasterized::Owned(raster) => draw_raster!(raster),
            }

            if cell.flags.contains(Flags::CURSOR_BEAM) {
                let (r, g, b) = color_cache.colors[0xff];
                (0..font_height)
                    .for_each(|y| self.graphic.draw_pixel(x_start, y_start + y, (r, g, b)));
            }

            if cell
                .flags
                .intersects(Flags::UNDERLINE | Flags::CURSOR_UNDERLINE)
            {
                let (r, g, b) = color_cache.colors[0xff];
                let y_base = y_start + font_height - 1;
                (0..font_width)
                    .for_each(|x| self.graphic.draw_pixel(x_start + x, y_base, (r, g, b)));
            }
        }
    }
}

struct ColorCache {
    colors: [Rgb; 256],
}

impl ColorCache {
    fn new(foreground: Rgb, background: Rgb) -> Self {
        let [r_diff, g_diff, b_diff] = [
            foreground.0 as i32 - background.0 as i32,
            foreground.1 as i32 - background.1 as i32,
            foreground.2 as i32 - background.2 as i32,
        ];

        let colors = core::array::from_fn(|intensity| {
            let weight = intensity as i32;
            (
                ((background.0 as i32 + (r_diff * weight / 0xff)).clamp(0, 255)) as u8,
                ((background.1 as i32 + (g_diff * weight / 0xff)).clamp(0, 255)) as u8,
                ((background.2 as i32 + (b_diff * weight / 0xff)).clamp(0, 255)) as u8,
            )
        });

        Self { colors }
    }
}
