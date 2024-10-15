use unicode_width::UnicodeWidthChar;

use super::color::{Color, NamedColor};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u16 {
        const INVERSE = 1 << 0;
        const BOLD = 1 << 1;
        const ITALIC = 1 << 2;
        const UNDERLINE = 1 << 3;
        const HIDDEN = 1 << 4;
        const CURSOR_BLOCK = 1 << 5;
        const CURSOR_UNDERLINE = 1 << 6;
        const CURSOR_BEAM = 1 << 7;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub content: char,
    pub flags: Flags,
    pub foreground: Color,
    pub background: Color,
    pub width_ratio: usize,
    pub placeholder: bool,
}

impl Cell {
    pub fn reset(&self) -> Self {
        Self {
            background: self.background,
            ..Default::default()
        }
    }

    pub fn with_placeholder(&self) -> Self {
        Self {
            placeholder: true,
            ..*self
        }
    }

    pub fn with_content(&self, content: char) -> Self {
        Self {
            content,
            width_ratio: content.width().unwrap(),
            ..*self
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            content: ' ',
            flags: Flags::empty(),
            foreground: Color::Indexed(NamedColor::BrightWhite as u8),
            background: Color::Indexed(NamedColor::Black as u8),
            width_ratio: 1,
            placeholder: false,
        }
    }
}
