use alloc::boxed::Box;
use core::mem::swap;
use core::num::NonZeroUsize;
use core::ops::{Deref, DerefMut};
use lru::LruCache;
use vte::ansi::Color;

use crate::cell::{Cell, Flags};
use crate::color::{ColorScheme, Rgb};
use crate::font::{ContentInfo, FontManager, Rasterized};

pub trait DrawTarget {
    fn size(&self) -> (usize, usize);
    fn draw_pixel(&mut self, x: usize, y: usize, pixel: u32);
    fn rgb_to_pixel(&self, rgb: Rgb) -> u32;
}

pub struct Graphic<D: DrawTarget> {
    display: D,
    pub(crate) color_scheme: ColorScheme,
    pub(crate) font_manager: Option<Box<dyn FontManager>>,
    color_cache: LruCache<(Rgb, Rgb), ColorCache>,
}

impl<D: DrawTarget> Deref for Graphic<D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        &self.display
    }
}

impl<D: DrawTarget> DerefMut for Graphic<D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.display
    }
}

impl<D: DrawTarget> Graphic<D> {
    pub fn new(display: D) -> Self {
        Self {
            display,
            color_scheme: ColorScheme::default(),
            font_manager: None,
            color_cache: LruCache::new(NonZeroUsize::new(128).unwrap()),
        }
    }

    pub fn set_cache_size(&mut self, size: usize) {
        assert!(size > 0, "Cache size must be greater than 0");
        self.color_cache.resize(NonZeroUsize::new(size).unwrap());
    }
}

impl<D: DrawTarget> Graphic<D> {
    pub fn clear(&mut self, cell: Cell) {
        let (width, height) = self.display.size();
        let rgb = self.color_to_rgb(cell.background);
        let pixel = self.display.rgb_to_pixel(rgb);

        for y in 0..height {
            for x in 0..width {
                self.display.draw_pixel(x, y, pixel);
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
                .get_or_insert((foreground, background), || {
                    ColorCache::new(foreground, background, &self.display)
                });

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
                            let pixel = color_cache.0[intensity as usize];
                            self.display.draw_pixel(x_start + x, y_start + y, pixel);
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
                let pixel = color_cache.0[0xff];
                (0..font_height)
                    .for_each(|y| self.display.draw_pixel(x_start, y_start + y, pixel));
            }

            if cell
                .flags
                .intersects(Flags::UNDERLINE | Flags::CURSOR_UNDERLINE)
            {
                let pixel = color_cache.0[0xff];
                let y_base = y_start + font_height - 1;
                (0..font_width)
                    .for_each(|x| self.display.draw_pixel(x_start + x, y_base, pixel));
            }
        }
    }
}

struct ColorCache([u32; 256]);

impl ColorCache {
    fn new<D: DrawTarget>(foreground: Rgb, background: Rgb, display: &D) -> Self {
        let (r_diff, g_diff, b_diff) = (
            foreground.0 as i32 - background.0 as i32,
            foreground.1 as i32 - background.1 as i32,
            foreground.2 as i32 - background.2 as i32,
        );

        let colors = core::array::from_fn(|intensity| {
            let weight = intensity as i32;
            
            let r = ((background.0 as i32 + (r_diff * weight / 0xff)).clamp(0, 255)) as u8;
            let g = ((background.1 as i32 + (g_diff * weight / 0xff)).clamp(0, 255)) as u8;
            let b = ((background.2 as i32 + (b_diff * weight / 0xff)).clamp(0, 255)) as u8;

            display.rgb_to_pixel((r, g, b))
        });

        Self(colors)
    }
}
