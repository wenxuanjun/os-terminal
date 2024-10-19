use crate::config::CONFIG;
use crate::palette::{DEFAULT_PALETTE_INDEX, PALETTE_DATA};

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

pub type Rgb888 = (u8, u8, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Indexed(u8),
    Rgb(Rgb888),
}

impl Color {
    pub fn to_rgb(self) -> Rgb888 {
        match self {
            Self::Rgb(rgb) => rgb,
            Self::Indexed(idx) => CONFIG.color_scheme.lock().0[idx as usize],
        }
    }
}

pub struct ColorScheme([Rgb888; 256]);

impl Default for ColorScheme {
    fn default() -> Self {
        Self::new(DEFAULT_PALETTE_INDEX)
    }
}

impl ColorScheme {
    pub fn new(palette_index: usize) -> Self {
        let mut colors = [(0, 0, 0); 256];
        let palette = PALETTE_DATA
            .get(palette_index)
            .unwrap_or(&PALETTE_DATA[DEFAULT_PALETTE_INDEX]);

        for (i, color) in palette.iter().enumerate() {
            colors[i] = *color;
        }

        for r_level in 0..6 {
            for g_level in 0..6 {
                for b_level in 0..6 {
                    let index = 16 + 36 * r_level + 6 * g_level + b_level;
                    let scale = |c: usize| if c == 0 { 0 } else { (c * 40 + 55) as u8 };
                    colors[index] = (scale(r_level), scale(g_level), scale(b_level));
                }
            }
        }

        for gray_level in 0..24 {
            let index = 16 + 216 + gray_level;
            let color_value = (gray_level * 10 + 8) as u8;
            colors[index] = (color_value, color_value, color_value);
        }

        Self(colors)
    }
}
