use alloc::vec::Vec;
use core::fmt;

use zune_png::error::PngDecodeErrors;
use zune_png::zune_core::bytestream::ZCursor;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::zune_core::options::DecoderOptions;
use zune_png::PngDecoder;

use crate::color::Rgb;

#[derive(Debug)]
pub enum WallpaperError {
    Decode(PngDecodeErrors),
    InvalidImage,
    UnsupportedColorSpace(ColorSpace),
}

impl From<PngDecodeErrors> for WallpaperError {
    fn from(error: PngDecodeErrors) -> Self {
        Self::Decode(error)
    }
}

impl fmt::Display for WallpaperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "PNG decode failed: {error:?}"),
            Self::InvalidImage => write!(f, "PNG metadata is invalid"),
            Self::UnsupportedColorSpace(color_space) => {
                write!(f, "unsupported PNG color space: {color_space:?}")
            }
        }
    }
}

pub(crate) struct Wallpaper {
    source_size: (usize, usize),
    source_pixels: Vec<u8>,
    scaled_size: (usize, usize),
    scaled_pixels: Vec<u8>,
}

impl Wallpaper {
    pub(crate) fn decode(
        png_data: &[u8],
        display_size: (usize, usize),
    ) -> Result<Self, WallpaperError> {
        let options = DecoderOptions::default()
            .png_set_strip_to_8bit(true)
            .png_set_add_alpha_channel(true)
            .png_set_decode_animated(false);

        let mut decoder = PngDecoder::new_with_options(ZCursor::new(png_data), options);
        let source_pixels = decoder.decode_raw()?;
        let source_size = decoder.dimensions().ok_or(WallpaperError::InvalidImage)?;
        let color_space = decoder.colorspace().ok_or(WallpaperError::InvalidImage)?;

        let source_pixels = Self::to_rgba(source_pixels, source_size, color_space)?;
        let mut wallpaper = Self {
            source_size,
            source_pixels,
            scaled_size: (0, 0),
            scaled_pixels: Vec::new(),
        };
        wallpaper.ensure_scaled(display_size);
        Ok(wallpaper)
    }

    pub(crate) fn ensure_scaled(&mut self, display_size: (usize, usize)) {
        if self.scaled_size == display_size {
            return;
        }

        let Some(len) = pixel_capacity(display_size, 4) else {
            self.scaled_size = (0, 0);
            self.scaled_pixels.clear();
            return;
        };

        self.scaled_size = display_size;
        self.scaled_pixels.resize(len, 0);

        let (dst_width, dst_height) = display_size;
        if dst_width == 0 || dst_height == 0 {
            return;
        }

        let (src_width, src_height) = self.source_size;
        for y in 0..dst_height {
            let src_y = y * src_height / dst_height;

            for x in 0..dst_width {
                let src_x = x * src_width / dst_width;
                let src_idx = (src_y * src_width + src_x) * 4;
                let dst_idx = (y * dst_width + x) * 4;
                self.scaled_pixels[dst_idx..dst_idx + 4]
                    .copy_from_slice(&self.source_pixels[src_idx..src_idx + 4]);
            }
        }
    }

    pub(crate) fn sample(&self, x: usize, y: usize, background: Rgb) -> Rgb {
        let (width, height) = self.scaled_size;
        if x >= width || y >= height {
            return background;
        }

        let idx = (y * width + x) * 4;
        blend_rgb(
            (
                self.scaled_pixels[idx],
                self.scaled_pixels[idx + 1],
                self.scaled_pixels[idx + 2],
            ),
            background,
            self.scaled_pixels[idx + 3],
        )
    }

    fn to_rgba(
        pixels: Vec<u8>,
        size: (usize, usize),
        color_space: ColorSpace,
    ) -> Result<Vec<u8>, WallpaperError> {
        let pixel_count = pixel_capacity(size, 1).ok_or(WallpaperError::InvalidImage)?;

        match color_space {
            ColorSpace::RGBA => {
                if pixels.len() == pixel_count * 4 {
                    Ok(pixels)
                } else {
                    Err(WallpaperError::InvalidImage)
                }
            }
            ColorSpace::RGB => {
                if pixels.len() != pixel_count * 3 {
                    return Err(WallpaperError::InvalidImage);
                }

                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for pixel in pixels.chunks_exact(3) {
                    rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
                }
                Ok(rgba)
            }
            ColorSpace::LumaA => {
                if pixels.len() != pixel_count * 2 {
                    return Err(WallpaperError::InvalidImage);
                }

                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for pixel in pixels.chunks_exact(2) {
                    rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]);
                }
                Ok(rgba)
            }
            ColorSpace::Luma => {
                if pixels.len() != pixel_count {
                    return Err(WallpaperError::InvalidImage);
                }

                let mut rgba = Vec::with_capacity(pixel_count * 4);
                for pixel in pixels {
                    rgba.extend_from_slice(&[pixel, pixel, pixel, 255]);
                }
                Ok(rgba)
            }
            _ => Err(WallpaperError::UnsupportedColorSpace(color_space)),
        }
    }
}

fn pixel_capacity(size: (usize, usize), channels: usize) -> Option<usize> {
    size.0
        .checked_mul(size.1)
        .and_then(|count| count.checked_mul(channels))
}

fn blend_rgb(foreground: Rgb, background: Rgb, alpha: u8) -> Rgb {
    (
        blend_channel(foreground.0, background.0, alpha),
        blend_channel(foreground.1, background.1, alpha),
        blend_channel(foreground.2, background.2, alpha),
    )
}

fn blend_channel(foreground: u8, background: u8, alpha: u8) -> u8 {
    let alpha = alpha as u16;
    let background_alpha = 255 - alpha;
    (((foreground as u16 * alpha) + (background as u16 * background_alpha) + 127) / 255) as u8
}
