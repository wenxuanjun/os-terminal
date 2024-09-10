use spin::Lazy;

#[repr(u8)]
#[derive(Debug)]
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
            Self::Indexed(idx) => COLOR_SCHEME[idx as usize],
        }
    }
}

static COLOR_SCHEME: Lazy<[Rgb888; 256]> = Lazy::new(|| {
    let mut colors = [(0, 0, 0); 256];
    colors[NamedColor::Black as usize] = (0x15, 0x15, 0x15);
    colors[NamedColor::Red as usize] = (0xac, 0x41, 0x42);
    colors[NamedColor::Green as usize] = (0x90, 0xa9, 0x59);
    colors[NamedColor::Yellow as usize] = (0xf4, 0xbf, 0x75);
    colors[NamedColor::Blue as usize] = (0x6a, 0x9f, 0xb5);
    colors[NamedColor::Magenta as usize] = (0xaa, 0x75, 0x9f);
    colors[NamedColor::Cyan as usize] = (0x75, 0xb5, 0xaa);
    colors[NamedColor::White as usize] = (0xd0, 0xd0, 0xd0);
    colors[NamedColor::BrightBlack as usize] = (0x50, 0x50, 0x50);
    colors[NamedColor::BrightRed as usize] = (0xac, 0x41, 0x42);
    colors[NamedColor::BrightGreen as usize] = (0x90, 0xa9, 0x59);
    colors[NamedColor::BrightYellow as usize] = (0xf4, 0xbf, 0x75);
    colors[NamedColor::BrightBlue as usize] = (0x6a, 0x9f, 0xb5);
    colors[NamedColor::BrightMagenta as usize] = (0xaa, 0x75, 0x9f);
    colors[NamedColor::BrightCyan as usize] = (0x75, 0xb5, 0xaa);
    colors[NamedColor::BrightWhite as usize] = (0xf5, 0xf5, 0xf5);

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

    colors
});
