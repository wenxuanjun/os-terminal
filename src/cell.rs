use crate::{color::Color, config::CONFIG};
use unicode_width::UnicodeWidthChar;

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

    pub fn reset_color(&mut self) -> Self {
        let color_scheme = CONFIG.color_scheme.lock();

        if let Color::Rgb(_) = self.foreground {
            self.foreground = Color::Rgb(color_scheme.foreground);
        }
        if let Color::Rgb(_) = self.background {
            self.background = Color::Rgb(color_scheme.background);
        }

        *self
    }
}

impl Default for Cell {
    fn default() -> Self {
        let color_scheme = CONFIG.color_scheme.lock();

        Self {
            content: ' ',
            wide: false,
            placeholder: false,
            flags: Flags::empty(),
            foreground: Color::Rgb(color_scheme.foreground),
            background: Color::Rgb(color_scheme.background),
        }
    }
}
