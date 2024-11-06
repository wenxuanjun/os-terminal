use alloc::boxed::Box;
use alloc::string::String;
use core::sync::atomic::Ordering;
use core::{cmp::min, fmt};

use crate::ansi::{Attr, CursorShape, Handler, Mode, Performer};
use crate::ansi::{LineClearMode, ScreenClearMode};
use crate::buffer::TerminalBuffer;
use crate::cell::{Cell, Flags};
use crate::color::ColorScheme;
use crate::config::CONFIG;
use crate::font::FontManager;
use crate::graphic::{DrawTarget, TextOnGraphic};
use crate::keyboard::{KeyboardEvent, KeyboardManager};
use crate::palette::Palette;

bitflags::bitflags! {
    pub struct TerminalMode: u32 {
        const SHOW_CURSOR = 1 << 0;
        const APP_CURSOR = 1 << 1;
        const APP_KEYPAD = 1 << 2;
        const MOUSE_REPORT_CLICK = 1 << 3;
        const BRACKETED_PASTE = 1 << 4;
        const SGR_MOUSE = 1 << 5;
        const MOUSE_MOTION = 1 << 6;
        const LINE_WRAP = 1 << 7;
        const LINE_FEED_NEW_LINE = 1 << 8;
        const ORIGIN = 1 << 9;
        const INSERT = 1 << 10;
        const FOCUS_IN_OUT = 1 << 11;
        const ALT_SCREEN = 1 << 12;
        const MOUSE_DRAG = 1 << 13;
        const MOUSE_MODE = 1 << 14;
        const UTF8_MOUSE = 1 << 15;
        const ALTERNATE_SCROLL = 1 << 16;
        const VI = 1 << 17;
        const URGENCY_HINTS = 1 << 18;
        const ANY = u32::MAX;
    }
}

impl Default for TerminalMode {
    fn default() -> TerminalMode {
        TerminalMode::SHOW_CURSOR | TerminalMode::LINE_WRAP
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Cursor {
    row: usize,
    column: usize,
    shape: CursorShape,
}

pub struct Terminal<D: DrawTarget> {
    parser: vte::Parser,
    inner: TerminalInner<D>,
}

pub struct TerminalInner<D: DrawTarget> {
    cursor: Cursor,
    saved_cursor: Cursor,
    mode: TerminalMode,
    attribute_template: Cell,
    buffer: TerminalBuffer<D>,
    keyboard: KeyboardManager,
}

impl<D: DrawTarget> Terminal<D> {
    pub fn new(display: D) -> Self {
        let mut graphic = TextOnGraphic::new(display);
        graphic.clear(Cell::default());

        Self {
            parser: vte::Parser::new(),
            inner: TerminalInner {
                cursor: Cursor::default(),
                saved_cursor: Cursor::default(),
                mode: TerminalMode::default(),
                attribute_template: Cell::default(),
                buffer: TerminalBuffer::new(graphic),
                keyboard: KeyboardManager::default(),
            },
        }
    }

    pub fn rows(&self) -> usize {
        self.inner.buffer.height()
    }

    pub fn columns(&self) -> usize {
        self.inner.buffer.width()
    }

    pub fn flush(&mut self) {
        self.inner.buffer.flush();
    }

    pub fn advance_state(&mut self, bstr: &[u8]) {
        self.inner.cursor_handler(false);
        let mut performer = Performer::new(&mut self.inner);
        for &byte in bstr {
            self.parser.advance(&mut performer, byte);
        }
        if self.inner.mode.contains(TerminalMode::SHOW_CURSOR) {
            self.inner.cursor_handler(true);
        }
        if CONFIG.auto_flush.load(Ordering::Relaxed) {
            self.flush();
        }
    }

    pub fn handle_keyboard(&mut self, scancode: u8) -> Option<String> {
        match self.inner.keyboard.handle_keyboard(scancode) {
            KeyboardEvent::AnsiString(s) => Some(s),
            KeyboardEvent::SetColorScheme(index) => {
                self.set_color_scheme(index);
                None
            }
            KeyboardEvent::None => None,
        }
    }
}

impl<D: DrawTarget> Terminal<D> {
    pub fn set_auto_flush(&mut self, auto_flush: bool) {
        CONFIG.auto_flush.store(auto_flush, Ordering::Relaxed);
    }

    pub fn set_logger(&mut self, logger: Option<fn(fmt::Arguments)>) {
        *CONFIG.logger.lock() = logger;
    }

    pub fn set_font_manager(&mut self, font_manager: Box<dyn FontManager>) {
        let (font_width, font_height) = font_manager.size();
        self.inner.buffer.update_size(font_width, font_height);
        *CONFIG.font_manager.lock() = Some(font_manager);
    }

    pub fn set_color_scheme(&mut self, palette_index: usize) {
        *CONFIG.color_scheme.lock() = ColorScheme::new(palette_index);
        self.inner.attribute_template = Cell::default();
        self.inner.buffer.full_flush();
    }

    pub fn set_custom_color_scheme(&mut self, palette: Palette) {
        *CONFIG.color_scheme.lock() = ColorScheme::from_palette(&palette);
        self.inner.attribute_template = Cell::default();
        self.inner.buffer.full_flush();
    }
}

impl<D: DrawTarget> fmt::Write for Terminal<D> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.advance_state(s.as_bytes());
        Ok(())
    }
}

impl<D: DrawTarget> TerminalInner<D> {
    fn cursor_handler(&mut self, enable: bool) {
        let row = self.cursor.row % self.buffer.height();
        let column = self.cursor.column % self.buffer.width();

        let mut origin_cell = self.buffer.read(row, column);

        let flag = match self.cursor.shape {
            CursorShape::Block => Flags::CURSOR_BLOCK,
            CursorShape::Underline => Flags::CURSOR_UNDERLINE,
            CursorShape::Beam => Flags::CURSOR_BEAM,
        };

        if enable {
            origin_cell.flags.insert(flag);
        } else {
            origin_cell.flags.remove(flag);
        }
        self.buffer.write(row, column, origin_cell);
    }
}

impl<D: DrawTarget> Handler for TerminalInner<D> {
    fn input(&mut self, content: char) {
        let template = self.attribute_template.with_content(content);

        if self.cursor.column + template.width_ratio > self.buffer.width() {
            if !self.mode.contains(TerminalMode::LINE_WRAP) {
                return;
            }
            self.cursor.column = 0;
            self.linefeed();
        }

        self.buffer
            .write(self.cursor.row, self.cursor.column, template);
        self.cursor.column += 1;

        for _ in 0..(template.width_ratio - 1) {
            self.buffer.write(
                self.cursor.row,
                self.cursor.column,
                template.with_placeholder(),
            );
            self.cursor.column += 1;
        }
    }

    fn goto(&mut self, row: usize, col: usize) {
        self.cursor.row = min(row, self.buffer.height());
        self.cursor.column = min(col, self.buffer.width());
    }

    fn goto_line(&mut self, row: usize) {
        self.goto(row, self.cursor.column);
    }

    fn goto_column(&mut self, col: usize) {
        self.goto(self.cursor.row, col);
    }

    fn move_up(&mut self, rows: usize) {
        self.goto(self.cursor.row.saturating_sub(rows), self.cursor.column);
    }

    fn move_down(&mut self, rows: usize) {
        let goto_line = min(self.cursor.row + rows, self.buffer.height() - 1) as _;
        self.goto(goto_line, self.cursor.column);
    }

    fn move_forward(&mut self, cols: usize) {
        self.cursor.column = min(self.cursor.column + cols, self.buffer.width() - 1);
    }

    fn move_backward(&mut self, cols: usize) {
        self.cursor.column = self.cursor.column.saturating_sub(cols);
    }

    fn move_up_and_cr(&mut self, rows: usize) {
        self.goto(self.cursor.row.saturating_sub(rows), 0);
    }

    fn move_down_and_cr(&mut self, rows: usize) {
        let goto_line = min(self.cursor.row + rows, self.buffer.height() - 1) as _;
        self.goto(goto_line, 0);
    }

    fn put_tab(&mut self) {
        let tab_stop = self.cursor.column.div_ceil(8) * 8;
        let end_column = tab_stop.min(self.buffer.width());
        let template = self.attribute_template.reset_content();

        while self.cursor.column < end_column {
            self.buffer
                .write(self.cursor.row, self.cursor.column, template);
            self.cursor.column += 1;
        }
    }

    fn backspace(&mut self) {
        self.cursor.column = self.cursor.column.saturating_sub(1);
    }

    fn carriage_return(&mut self) {
        self.cursor.column = 0;
    }

    fn linefeed(&mut self) {
        self.cursor.column = 0;
        if self.cursor.row < self.buffer.height() - 1 {
            self.cursor.row += 1;
        } else {
            self.buffer.new_line(self.attribute_template);
        }
    }

    fn erase_chars(&mut self, count: usize) {
        let start = self.cursor.column;
        let end = min(start + count, self.buffer.width());

        let template = self.attribute_template.reset_content();
        for column in start..end {
            self.buffer.write(self.cursor.row, column, template);
        }
    }

    fn delete_chars(&mut self, count: usize) {
        let (row, columns) = (self.cursor.row, self.buffer.width());
        let count = min(count, columns - self.cursor.column - 1);

        let template = self.attribute_template.reset_content();
        for column in (self.cursor.column + count)..columns {
            self.buffer
                .write(row, column - count, self.buffer.read(row, column));
            self.buffer.write(row, column, template);
        }
    }

    fn save_cursor_position(&mut self) {
        self.saved_cursor = self.cursor;
    }

    fn restore_cursor_position(&mut self) {
        self.cursor = self.saved_cursor;
    }

    fn set_cursor_shape(&mut self, shape: CursorShape) {
        self.cursor.shape = shape;
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        let (start, end) = match mode {
            LineClearMode::Right => (self.cursor.column, self.buffer.width()),
            LineClearMode::Left => (0, self.cursor.column + 1),
            LineClearMode::All => (0, self.buffer.width()),
        };
        let template = self.attribute_template.reset_content();
        for column in start..end {
            self.buffer.write(self.cursor.row, column, template);
        }
    }

    fn clear_screen(&mut self, mode: ScreenClearMode) {
        let template = self.attribute_template.reset_content();
        match mode {
            ScreenClearMode::Above => {
                for row in 0..self.cursor.row {
                    for column in 0..self.buffer.width() {
                        self.buffer.write(row, column, template);
                    }
                }
                for column in 0..self.cursor.column {
                    self.buffer.write(self.cursor.row, column, template);
                }
            }
            ScreenClearMode::Below => {
                for column in self.cursor.column..self.buffer.width() {
                    self.buffer.write(self.cursor.row, column, template);
                }
                for row in self.cursor.row + 1..self.buffer.height() {
                    for column in 0..self.buffer.width() {
                        self.buffer.write(row, column, template);
                    }
                }
            }
            ScreenClearMode::All => {
                self.buffer.clear(template);
                self.cursor = Cursor::default();
            }
            _ => {}
        }
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        match attr {
            Attr::Foreground(color) => self.attribute_template.foreground = color,
            Attr::Background(color) => self.attribute_template.background = color,
            Attr::Reset => self.attribute_template = Cell::default(),
            Attr::Reverse => self.attribute_template.flags |= Flags::INVERSE,
            Attr::CancelReverse => self.attribute_template.flags.remove(Flags::INVERSE),
            Attr::Bold => self.attribute_template.flags.insert(Flags::BOLD),
            Attr::CancelBold => self.attribute_template.flags.remove(Flags::BOLD),
            Attr::CancelBoldDim => self.attribute_template.flags.remove(Flags::BOLD),
            Attr::Italic => self.attribute_template.flags.insert(Flags::ITALIC),
            Attr::CancelItalic => self.attribute_template.flags.remove(Flags::ITALIC),
            Attr::Underline => self.attribute_template.flags.insert(Flags::UNDERLINE),
            Attr::CancelUnderline => self.attribute_template.flags.remove(Flags::UNDERLINE),
            Attr::Hidden => self.attribute_template.flags.insert(Flags::HIDDEN),
            Attr::CancelHidden => self.attribute_template.flags.remove(Flags::HIDDEN),
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        match mode {
            Mode::ShowCursor => self.mode.insert(TerminalMode::SHOW_CURSOR),
            Mode::CursorKeys => {
                self.mode.insert(TerminalMode::APP_CURSOR);
                self.keyboard.set_app_cursor(true);
            }
            Mode::LineWrap => self.mode.insert(TerminalMode::LINE_WRAP),
            _ => log!("Unhandled set mode: {:?}", mode),
        }
    }

    #[inline]
    fn unset_mode(&mut self, mode: Mode) {
        match mode {
            Mode::ShowCursor => self.mode.remove(TerminalMode::SHOW_CURSOR),
            Mode::CursorKeys => {
                self.mode.remove(TerminalMode::APP_CURSOR);
                self.keyboard.set_app_cursor(false);
            }
            Mode::LineWrap => self.mode.remove(TerminalMode::LINE_WRAP),
            _ => log!("Unhandled unset mode: {:?}", mode),
        }
    }
}
