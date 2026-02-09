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
    fn draw_pixel(&mut self, x: usize, y: usize, rgb: Rgb);
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

        for y in 0..height {
            for x in 0..width {
                self.display.draw_pixel(x, y, rgb);
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
                    ColorCache::new(foreground, background)
                });

            let content_info = ContentInfo {
                content: cell.content,
                bold: cell.flags.contains(Flags::BOLD),
                italic: cell.flags.contains(Flags::ITALIC),
                wide: cell.wide,
            };

            macro_rules! draw_gray_raster {
                ($raster:ident) => {
                    for (y, line_data) in $raster.iter().enumerate() {
                        for (x, &alpha) in line_data.iter().enumerate() {
                            let rgb = color_cache.to_rgb(alpha);
                            self.display.draw_pixel(x_start + x, y_start + y, rgb);
                        }
                    }
                };
            }

            macro_rules! draw_subpixel_raster {
                ($raster:ident) => {
                    for (y, line_data) in $raster.iter().enumerate() {
                        for (x, [r, g, b]) in line_data.iter().enumerate() {
                            let rgb = color_cache.to_subpixel(*r, *g, *b);
                            self.display.draw_pixel(x_start + x, y_start + y, rgb);
                        }
                    }
                };
            }

            match font_manager.rasterize(content_info) {
                Rasterized::GraySlice(raster) => draw_gray_raster!(raster),
                Rasterized::GrayVec(raster) => draw_gray_raster!(raster),
                Rasterized::SubpixelVec(raster) => draw_subpixel_raster!(raster),
            }

            if cell.flags.contains(Flags::CURSOR_BEAM) {
                let rgb = color_cache.to_rgb(255);
                (0..font_height).for_each(|y| self.display.draw_pixel(x_start, y_start + y, rgb));
            }

            if cell
                .flags
                .intersects(Flags::UNDERLINE | Flags::CURSOR_UNDERLINE)
            {
                let rgb = color_cache.to_rgb(255);
                let y_base = y_start + font_height - 1;
                (0..font_width).for_each(|x| self.display.draw_pixel(x_start + x, y_base, rgb));
            }
        }
    }
}

struct ColorCache {
    r_lut: [u8; 256],
    g_lut: [u8; 256],
    b_lut: [u8; 256],
}

impl ColorCache {
    fn to_rgb(&self, alpha: u8) -> Rgb {
        (
            self.r_lut[alpha as usize],
            self.g_lut[alpha as usize],
            self.b_lut[alpha as usize],
        )
    }

    fn to_subpixel(&self, red: u8, green: u8, blue: u8) -> Rgb {
        (
            self.r_lut[red as usize],
            self.g_lut[green as usize],
            self.b_lut[blue as usize],
        )
    }

    fn new(foreground: Rgb, background: Rgb) -> Self {
        let gen_lut = |fg: u8, bg: u8| -> [u8; 256] {
            let foreground = fg as i32;
            let background = bg as i32;
            let different = foreground - background;

            core::array::from_fn(|intensity| {
                (background + different * intensity as i32 / 255) as u8
            })
        };

        Self {
            r_lut: gen_lut(foreground.0, background.0),
            g_lut: gen_lut(foreground.1, background.1),
            b_lut: gen_lut(foreground.2, background.2),
        }
    }
}
