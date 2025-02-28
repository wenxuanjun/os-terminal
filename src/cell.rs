use unicode_width::UnicodeWidthChar;
use vte::ansi::{Color, NamedColor};

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u8 {
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
    pub wide: bool,
    pub placeholder: bool,
    pub flags: Flags,
    pub foreground: Color,
    pub background: Color,
}

impl Cell {
    pub fn set_placeholder(mut self) -> Self {
        self.placeholder = true;
        self
    }

    pub fn set_content(mut self, content: char) -> Self {
        self.content = content;
        self.wide = content.width().unwrap_or(0) > 1;
        self
    }

    pub fn clear(&self) -> Self {
        Self {
            background: self.background,
            foreground: self.foreground,
            ..Default::default()
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            content: ' ',
            wide: false,
            placeholder: false,
            flags: Flags::empty(),
            foreground: Color::Named(NamedColor::Foreground),
            background: Color::Named(NamedColor::Background),
        }
    }
}
