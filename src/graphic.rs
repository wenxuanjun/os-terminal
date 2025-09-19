use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use core::mem::swap;
use core::ops::{Deref, DerefMut};
use vte::ansi::Color;

use crate::cell::{Cell, Flags};
use crate::color::{ColorScheme, Rgb};
use crate::font::{ContentInfo, FontManager, Rasterized};

pub trait DrawTarget {
    fn size(&self) -> (usize, usize);
    fn draw_pixel(&mut self, x: usize, y: usize, color: Rgb);
}

pub struct Graphic<D: DrawTarget> {
    graphic: D,
    pub(crate) color_scheme: ColorScheme,
    pub(crate) font_manager: Option<Box<dyn FontManager>>,
    color_cache: BTreeMap<(Rgb, Rgb), ColorCache>,
}

impl<D: DrawTarget> Deref for Graphic<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.graphic
    }
}

impl<D: DrawTarget> DerefMut for Graphic<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.graphic
    }
}

impl<D: DrawTarget> Graphic<D> {
    pub fn new(graphic: D) -> Self {
        Self {
            graphic,
            color_scheme: ColorScheme::default(),
            font_manager: None,
            color_cache: BTreeMap::new(),
        }
    }

    pub fn clear(&mut self, cell: Cell) {
        let color = self.color_to_rgb(cell.background);

        for y in 0..self.graphic.size().1 {
            for x in 0..self.graphic.size().0 {
                self.graphic.draw_pixel(x, y, color);
            }
        }
    }

    pub fn color_to_rgb(&self, color: Color) -> Rgb {
        match color {
            Color::Spec(rgb) => (rgb.r, rgb.g, rgb.b),
            Color::Named(color) => match color as usize {
                256 => self.color_scheme.foreground,
                257 => self.color_scheme.background,
                index => self.color_scheme.ansi_colors[index],
            },
            Color::Indexed(index) => {
                let color_scheme = &self.color_scheme;
                color_scheme.ansi_colors[index as usize]
            }
        }
    }
}

impl<D: DrawTarget> Graphic<D> {
    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        if cell.placeholder {
            return;
        }

        let mut foreground = self.color_to_rgb(cell.foreground);
        let mut background = self.color_to_rgb(cell.background);

        if cell.flags.intersects(Flags::INVERSE | Flags::CURSOR_BLOCK) {
            swap(&mut foreground, &mut background);
        }

        if cell.flags.contains(Flags::HIDDEN) {
            foreground = background;
        }

        if let Some(font_manager) = self.font_manager.as_mut() {
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
                            let (r, g, b) = color_cache.0[intensity as usize];
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
                let (r, g, b) = color_cache.0[0xff];
                (0..font_height)
                    .for_each(|y| self.graphic.draw_pixel(x_start, y_start + y, (r, g, b)));
            }

            if cell
                .flags
                .intersects(Flags::UNDERLINE | Flags::CURSOR_UNDERLINE)
            {
                let (r, g, b) = color_cache.0[0xff];
                let y_base = y_start + font_height - 1;
                (0..font_width)
                    .for_each(|x| self.graphic.draw_pixel(x_start + x, y_base, (r, g, b)));
            }
        }
    }
}

struct ColorCache([Rgb; 256]);

impl ColorCache {
    fn new(foreground: Rgb, background: Rgb) -> Self {
        let (r_diff, g_diff, b_diff) = (
            foreground.0 as i32 - background.0 as i32,
            foreground.1 as i32 - background.1 as i32,
            foreground.2 as i32 - background.2 as i32,
        );

        let colors = core::array::from_fn(|intensity| {
            let weight = intensity as i32;
            (
                ((background.0 as i32 + (r_diff * weight / 0xff)).clamp(0, 255)) as u8,
                ((background.1 as i32 + (g_diff * weight / 0xff)).clamp(0, 255)) as u8,
                ((background.2 as i32 + (b_diff * weight / 0xff)).clamp(0, 255)) as u8,
            )
        });

        Self(colors)
    }
}
