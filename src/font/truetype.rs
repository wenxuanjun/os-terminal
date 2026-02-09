use alloc::vec::Vec;
use core::num::NonZeroUsize;
use lru::LruCache;
use swash::scale::{image::Content, Render, ScaleContext, Source};
use swash::zeno::{Angle, Format, Transform};
use swash::FontRef;

use super::{ContentInfo, FontManager, Rasterized};

pub enum RasterBuffer {
    Gray(Vec<Vec<u8>>),
    Subpixel(Vec<Vec<[u8; 3]>>),
}

pub struct TrueTypeFont {
    font: FontRef<'static>,
    italic_font: Option<FontRef<'static>>,
    subpixel: bool,
    scale_context: ScaleContext,
    raster_height: usize,
    raster_width: usize,
    font_size: f32,
    base_line_offset: f32,
    bitmap_cache: LruCache<ContentInfo, RasterBuffer>,
}

impl TrueTypeFont {
    pub fn new(font_size: f32, font_bytes: &'static [u8]) -> Self {
        let font = FontRef::from_index(font_bytes, 0).unwrap();
        let font_size = font_size * 96.0 / 72.0;
        let metrics = font.metrics(&[]).scale(font_size);
        let line_height = metrics.ascent + metrics.descent + metrics.leading;

        Self {
            font,
            italic_font: None,
            subpixel: false,
            raster_height: line_height as usize,
            raster_width: (line_height / 2.0) as usize,
            font_size,
            base_line_offset: metrics.ascent,
            scale_context: ScaleContext::default(),
            bitmap_cache: LruCache::new(NonZeroUsize::new(512).unwrap()),
        }
    }

    pub fn with_subpixel(mut self, enabled: bool) -> Self {
        self.subpixel = enabled;
        self
    }

    pub fn with_cache_size(mut self, size: usize) -> Self {
        assert!(size > 0, "Cache size must be greater than 0");
        self.bitmap_cache.resize(NonZeroUsize::new(size).unwrap());
        self
    }

    pub fn with_italic_font(mut self, italic_font: &'static [u8]) -> Self {
        self.italic_font = Some(FontRef::from_index(italic_font, 0).unwrap());
        self
    }
}

impl FontManager for TrueTypeFont {
    fn size(&self) -> (usize, usize) {
        (self.raster_width, self.raster_height)
    }

    fn rasterize(&mut self, info: ContentInfo) -> Rasterized<'_> {
        let bitmap = self.bitmap_cache.get_or_insert(info.clone(), || {
            let (select_font, need_skew) = self
                .italic_font
                .as_ref()
                .filter(|_| info.italic)
                .map(|f| (f, false))
                .unwrap_or((&self.font, info.italic));

            let weight_tag = u32::from_be_bytes(*b"wght");
            let has_weight_axis = select_font.variations().any(|v| v.tag() == weight_tag);
            let font_weight = if info.bold { 700.0 } else { 400.0 };

            let mut scaler = self
                .scale_context
                .builder(*select_font)
                .size(self.font_size)
                .variations(&[("wght", font_weight)])
                .hint(true)
                .build();

            let mut renderer = Render::new(&[Source::Outline]);

            if info.bold && !has_weight_axis {
                renderer.embolden(1.0);
            }

            if need_skew {
                let skew_x = Angle::from_radians(0.25);
                let skew_y = Angle::ZERO;
                renderer.transform(Some(Transform::skew(skew_x, skew_y)));
            }

            if self.subpixel {
                renderer.format(Format::Subpixel);
            }

            let glyph_id = select_font.charmap().map(info.content);
            let width = self.raster_width * if info.wide { 2 } else { 1 };
            let make_gray = || vec![vec![0u8; width]; self.raster_height];
            let make_subpixel = || vec![vec![[0u8; 3]; width]; self.raster_height];

            let Some(image) = renderer.render(&mut scaler, glyph_id) else {
                return if self.subpixel {
                    RasterBuffer::Subpixel(make_subpixel())
                } else {
                    RasterBuffer::Gray(make_gray())
                };
            };

            let img_w = image.placement.width as i32;
            let img_h = image.placement.height as i32;
            let y_offset = self.base_line_offset as i32 - image.placement.top;
            let x_offset = image.placement.left;

            let y_min = 0.max(-y_offset);
            let y_max = img_h.min(self.raster_height as i32 - y_offset);

            let x_min = 0.max(-x_offset);
            let x_max = img_w.min(width as i32 - x_offset);

            if self.subpixel && matches!(image.content, Content::SubpixelMask) {
                let mut letter_bitmap = make_subpixel();

                for y in y_min..y_max {
                    let dst_row = &mut letter_bitmap[(y_offset + y) as usize];
                    let src_row_start = (y as usize) * (img_w as usize) * 4;

                    for x in x_min..x_max {
                        let src_idx = src_row_start + (x as usize) * 4;
                        dst_row[(x_offset + x) as usize] = [
                            image.data[src_idx],
                            image.data[src_idx + 1],
                            image.data[src_idx + 2],
                        ];
                    }
                }
                RasterBuffer::Subpixel(letter_bitmap)
            } else {
                let mut letter_bitmap = make_gray();

                for y in y_min..y_max {
                    let dst_row = &mut letter_bitmap[(y_offset + y) as usize];
                    let src_row_start = (y as usize) * (img_w as usize);

                    let dst_start = (x_offset + x_min) as usize;
                    let dst_end = (x_offset + x_max) as usize;
                    let src_start = src_row_start + x_min as usize;
                    let src_end = src_row_start + x_max as usize;

                    dst_row[dst_start..dst_end].copy_from_slice(&image.data[src_start..src_end]);
                }
                RasterBuffer::Gray(letter_bitmap)
            }
        });

        match bitmap {
            RasterBuffer::Gray(bitmap) => Rasterized::GrayVec(bitmap),
            RasterBuffer::Subpixel(bitmap) => Rasterized::SubpixelVec(bitmap),
        }
    }
}
