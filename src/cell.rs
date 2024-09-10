use super::color::{Color, NamedColor};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u16 {
        const INVERSE = 0b0000_0000_0001;
        const BOLD = 0b0000_0000_0010;
        const ITALIC = 0b0000_0000_0100;
        const UNDERLINE = 0b0000_0000_1000;
        const HIDDEN = 0b0000_0001_0000;
        const CURSOR_BLOCK = 0b0000_0010_0000;
        const CURSOR_UNDERLINE = 0b0000_0100_0000;
        const CURSOR_BEAM = 0b0000_1000_0000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub content: char,
    pub flags: Flags,
    pub foreground: Color,
    pub background: Color,
}

impl Cell {
    pub fn reset(&self) -> Self {
        Self {
            background: self.background,
            ..Default::default()
        }
    }

    pub const fn with_content(&self, content: char) -> Self {
        Self { content, ..*self }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            content: ' ',
            flags: Flags::empty(),
            foreground: Color::Indexed(NamedColor::BrightWhite as u8),
            background: Color::Indexed(NamedColor::Black as u8),
        }
    }
}
