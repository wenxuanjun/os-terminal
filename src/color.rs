use crate::config::CONFIG;
use crate::graphic::FgBgPair;
use crate::palette::{Palette, DEFAULT_PALETTE_INDEX, PALETTE_DATA};

#[repr(u8)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

pub type Rgb = (u8, u8, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Indexed(u16),
    Rgb(Rgb),
}

impl Color {
    pub fn to_rgb(self) -> Rgb {
        match self {
            Self::Rgb(rgb) => rgb,
            Self::Indexed(index) => {
                let color_scheme = CONFIG.color_scheme.lock();
                match index {
                    256 => color_scheme.color_pair.0,
                    257 => color_scheme.color_pair.1,
                    index => color_scheme.ansi_colors[index as usize],
                }
            }
        }
    }
}

pub struct ColorScheme {
    pub color_pair: FgBgPair,
    pub ansi_colors: [Rgb; 256],
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::new(DEFAULT_PALETTE_INDEX)
    }
}

impl ColorScheme {
    pub fn new(palette_index: usize) -> Self {
        let palette = PALETTE_DATA
            .get(palette_index)
            .unwrap_or(&PALETTE_DATA[DEFAULT_PALETTE_INDEX]);
        Self::from_palette(palette)
    }

    pub fn from_palette(palette: &Palette) -> Self {
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
            color_pair: palette.color_pair,
            ansi_colors: colors,
        }
    }
}
