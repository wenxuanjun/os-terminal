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
    pub flags: Flags,
    pub foreground: Color,
    pub background: Color,
    pub width_ratio: usize,
    pub placeholder: bool,
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
            width_ratio: content.width().unwrap(),
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
        let color_scheme = CONFIG.color_scheme.lock();

        let update_color = |current_color, new_color| match current_color {
            Color::Rgb(_) => Color::Rgb(new_color),
            _ => current_color,
        };

        self.foreground = update_color(self.foreground, color_scheme.foreground);
        self.background = update_color(self.background, color_scheme.background);

        *self
    }
}

impl Default for Cell {
    fn default() -> Self {
        let color_scheme = CONFIG.color_scheme.lock();

        Self {
            content: ' ',
            flags: Flags::empty(),
            foreground: Color::Rgb(color_scheme.foreground),
            background: Color::Rgb(color_scheme.background),
            width_ratio: 1,
            placeholder: false,
        }
    }
}
