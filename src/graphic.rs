use alloc::boxed::Box;
use core::mem::swap;
use core::num::NonZeroUsize;
use core::ops::{Deref, DerefMut};
use lru::LruCache;
use vte::ansi::{Color, NamedColor};

use crate::cell::{Cell, Flags};
use crate::color::{ColorScheme, Rgb};
use crate::font::{ContentInfo, FontManager, Rasterized};
#[cfg(feature = "wallpaper")]
use crate::wallpaper::{Wallpaper, WallpaperError};

pub trait DrawTarget {
    fn size(&self) -> (usize, usize);
    fn draw_pixel(&mut self, x: usize, y: usize, rgb: Rgb);
}

pub struct Graphic<D: DrawTarget> {
    display: D,
    pub(crate) color_scheme: ColorScheme,
    pub(crate) font_manager: Box<dyn FontManager>,
    color_cache: LruCache<(Rgb, Rgb), ColorCache>,
    #[cfg(feature = "wallpaper")]
    wallpaper: Option<Wallpaper>,
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
    pub fn new(display: D, font_manager: Box<dyn FontManager>) -> Self {
        Self {
            display,
            color_scheme: ColorScheme::default(),
            font_manager,
            color_cache: LruCache::new(NonZeroUsize::new(128).unwrap()),
            #[cfg(feature = "wallpaper")]
            wallpaper: None,
        }
    }

    pub fn set_cache_size(&mut self, size: usize) {
        assert!(size > 0, "Cache size must be greater than 0");
        self.color_cache.resize(NonZeroUsize::new(size).unwrap());
    }

    #[cfg(feature = "wallpaper")]
    pub fn set_wallpaper(&mut self, png_data: &[u8]) -> Result<(), WallpaperError> {
        let wallpaper = Wallpaper::decode(png_data, self.size())?;
        self.wallpaper = Some(wallpaper);
        Ok(())
    }

    #[cfg(feature = "wallpaper")]
    pub fn clear_wallpaper(&mut self) {
        self.wallpaper = None;
    }
}

impl<D: DrawTarget> Graphic<D> {
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
    fn has_wallpaper(&self) -> bool {
        #[cfg(feature = "wallpaper")]
        {
            return self.wallpaper.is_some();
        }

        #[cfg(not(feature = "wallpaper"))]
        {
            false
        }
    }

    pub fn prepare_frame(&mut self) {
        #[cfg(feature = "wallpaper")]
        if let Some(wallpaper) = self.wallpaper.as_mut() {
            let display_size = self.display.size();
            wallpaper.ensure_scaled(display_size);
        }
    }

    pub(crate) fn background_pixel(&self, _x: usize, _y: usize, fallback: Rgb) -> Rgb {
        #[cfg(feature = "wallpaper")]
        {
            if let Some(wallpaper) = self.wallpaper.as_ref() {
                return wallpaper.sample(_x, _y, fallback);
            }
        }

        fallback
    }
}

impl<D: DrawTarget> Graphic<D> {
    pub fn write(&mut self, row: usize, col: usize, cell: Cell) {
        if cell.placeholder {
            return;
        }

        let mut foreground_color = cell.foreground;
        let mut background_color = cell.background;

        if cell.flags.intersects(Flags::INVERSE | Flags::CURSOR_BLOCK) {
            swap(&mut foreground_color, &mut background_color);
        }

        let mut foreground = self.color_to_rgb(foreground_color);
        let background = self.color_to_rgb(background_color);
        let use_wallpaper = self.has_wallpaper()
            && matches!(background_color, Color::Named(NamedColor::Background));
        let hidden = cell.flags.contains(Flags::HIDDEN);

        if hidden && !use_wallpaper {
            foreground = background;
        }

        #[cfg(feature = "wallpaper")]
        let wallpaper = self.wallpaper.as_ref();
        let display = &mut self.display;
        let color_cache = &mut self.color_cache;
        let font_manager = self.font_manager.as_mut();
        let (font_width, font_height) = font_manager.size();
        let (x_start, y_start) = (col * font_width, row * font_height);

        let content_info = ContentInfo {
            content: cell.content,
            bold: cell.flags.contains(Flags::BOLD),
            italic: cell.flags.contains(Flags::ITALIC),
            wide: cell.wide,
        };

        macro_rules! draw_gray_raster {
            ($raster:ident) => {
                if use_wallpaper {
                    #[cfg(feature = "wallpaper")]
                    for (y, line_data) in $raster.iter().enumerate() {
                        for (x, &alpha) in line_data.iter().enumerate() {
                            let background_pixel =
                                wallpaper
                                    .unwrap()
                                    .sample(x_start + x, y_start + y, background);
                            let rgb = if hidden {
                                background_pixel
                            } else {
                                blend_rgb(foreground, background_pixel, alpha)
                            };
                            display.draw_pixel(x_start + x, y_start + y, rgb);
                        }
                    }
                } else {
                    let color_cache = color_cache.get_or_insert((foreground, background), || {
                        ColorCache::new(foreground, background)
                    });

                    for (y, line_data) in $raster.iter().enumerate() {
                        for (x, &alpha) in line_data.iter().enumerate() {
                            let rgb = color_cache.to_rgb(alpha);
                            display.draw_pixel(x_start + x, y_start + y, rgb);
                        }
                    }
                }
            };
        }

        macro_rules! draw_subpixel_raster {
            ($raster:ident) => {
                if use_wallpaper {
                    #[cfg(feature = "wallpaper")]
                    for (y, line_data) in $raster.iter().enumerate() {
                        for (x, [r, g, b]) in line_data.iter().enumerate() {
                            let background_pixel =
                                wallpaper
                                    .unwrap()
                                    .sample(x_start + x, y_start + y, background);
                            let rgb = if hidden {
                                background_pixel
                            } else {
                                (
                                    blend_channel(foreground.0, background_pixel.0, *r),
                                    blend_channel(foreground.1, background_pixel.1, *g),
                                    blend_channel(foreground.2, background_pixel.2, *b),
                                )
                            };
                            display.draw_pixel(x_start + x, y_start + y, rgb);
                        }
                    }
                } else {
                    let color_cache = color_cache.get_or_insert((foreground, background), || {
                        ColorCache::new(foreground, background)
                    });

                    for (y, line_data) in $raster.iter().enumerate() {
                        for (x, [r, g, b]) in line_data.iter().enumerate() {
                            let rgb = color_cache.to_subpixel(*r, *g, *b);
                            display.draw_pixel(x_start + x, y_start + y, rgb);
                        }
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
            let rgb = foreground;
            (0..font_height).for_each(|y| display.draw_pixel(x_start, y_start + y, rgb));
        }

        if cell
            .flags
            .intersects(Flags::UNDERLINE | Flags::CURSOR_UNDERLINE)
        {
            let rgb = foreground;
            let y_base = y_start + font_height - 1;
            (0..font_width).for_each(|x| display.draw_pixel(x_start + x, y_base, rgb));
        }
    }
}

#[cfg(feature = "wallpaper")]
fn blend_rgb(foreground: Rgb, background: Rgb, alpha: u8) -> Rgb {
    (
        blend_channel(foreground.0, background.0, alpha),
        blend_channel(foreground.1, background.1, alpha),
        blend_channel(foreground.2, background.2, alpha),
    )
}

#[cfg(feature = "wallpaper")]
fn blend_channel(foreground: u8, background: u8, alpha: u8) -> u8 {
    let alpha = alpha as u16;
    let background_alpha = 255 - alpha;
    (((foreground as u16 * alpha) + (background as u16 * background_alpha) + 127) / 255) as u8
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
