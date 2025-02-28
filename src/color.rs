use vte::ansi::Color;

use crate::config::CONFIG;
use crate::palette::{DEFAULT_PALETTE_INDEX, PALETTE, Palette};

pub type Rgb = (u8, u8, u8);

pub trait ToRgb {
    fn to_rgb(self) -> Rgb;
}

impl ToRgb for Color {
    fn to_rgb(self) -> Rgb {
        match self {
            Self::Spec(rgb) => (rgb.r, rgb.g, rgb.b),
            Self::Named(color) => {
                let color_scheme = CONFIG.color_scheme.lock();
                match color as usize {
                    256 => color_scheme.foreground,
                    257 => color_scheme.background,
                    index => color_scheme.ansi_colors[index],
                }
            }
            Self::Indexed(index) => {
                let color_scheme = CONFIG.color_scheme.lock();
                color_scheme.ansi_colors[index as usize]
            }
        }
    }
}

pub struct ColorScheme {
    pub foreground: Rgb,
    pub background: Rgb,
    pub ansi_colors: [Rgb; 256],
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::new(DEFAULT_PALETTE_INDEX)
    }
}

impl ColorScheme {
    pub fn new(palette_index: usize) -> Self {
        let palette = PALETTE
            .get(palette_index)
            .unwrap_or(&PALETTE[DEFAULT_PALETTE_INDEX]);
        ColorScheme::from(palette)
    }
}

impl From<&Palette> for ColorScheme {
    fn from(palette: &Palette) -> Self {
        let mut colors = [(0, 0, 0); 256];
        colors[..16].copy_from_slice(&palette.ansi_colors);

        for index in 0..216 {
            let r = index / 36;
            let g = (index % 36) / 6;
            let b = index % 6;
            let scale = |c: usize| if c == 0 { 0 } else { (c * 40 + 55) as u8 };
            colors[index + 16] = (scale(r), scale(g), scale(b));
        }

        for gray_level in 0..24 {
            let index = 16 + 216 + gray_level;
            let color_value = (gray_level * 10 + 8) as u8;
            colors[index] = (color_value, color_value, color_value);
        }

        Self {
            foreground: palette.foreground,
            background: palette.background,
            ansi_colors: colors,
        }
    }
}
