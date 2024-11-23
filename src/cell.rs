use unicode_width::UnicodeWidthChar;

use crate::{color::Color, config::CONFIG};

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
    pub fn with_placeholder(&self) -> Self {
        Self {
            placeholder: true,
            ..*self
        }
    }

    pub fn with_content(&self, content: char) -> Self {
        Self {
            content,
            wide: content.width().unwrap_or(0) > 1,
            ..*self
        }
    }

    pub fn reset_content(&self) -> Self {
        Self {
            background: self.background,
            foreground: self.foreground,
            ..Default::default()
        }
    }

    pub fn reset_color(&mut self) -> Self {
        let color_pair = CONFIG.color_scheme.lock().color_pair;

        if let Color::Rgb(_) = self.foreground {
            self.foreground = Color::Rgb(color_pair.0);
        }
        if let Color::Rgb(_) = self.background {
            self.background = Color::Rgb(color_pair.1);
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
            foreground: Color::Rgb(color_scheme.color_pair.0),
            background: Color::Rgb(color_scheme.color_pair.1),
        }
    }
}
