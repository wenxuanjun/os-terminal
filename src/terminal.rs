use alloc::boxed::Box;
use alloc::string::String;
use core::mem::swap;
use core::ops::Range;
use core::sync::atomic::Ordering;
use core::time::Duration;
use core::{cmp::min, fmt};
use pc_keyboard::KeyCode;

use vte::ansi::{Attr, Color as AnsiColor, NamedMode, Rgb};
use vte::ansi::{CharsetIndex, StandardCharset, TabulationClearMode};
use vte::ansi::{ClearMode, CursorShape, Processor, Timeout};
use vte::ansi::{CursorStyle, Hyperlink, KeyboardModes};
use vte::ansi::{Handler, LineClearMode, Mode, NamedPrivateMode, PrivateMode};

use crate::buffer::TerminalBuffer;
use crate::cell::{Cell, Flags};
use crate::color::{Color, ColorScheme};
use crate::config::CONFIG;
use crate::font::FontManager;
use crate::graphic::{DrawTarget, Graphic};
use crate::keyboard::{KeyboardEvent, KeyboardManager};
use crate::mouse::{MouseEvent, MouseInput, MouseManager};
use crate::palette::Palette;

#[derive(Default)]
pub struct DummySyncHandler;

#[rustfmt::skip]
impl Timeout for DummySyncHandler {
    fn set_timeout(&mut self, _duration: Duration) {}
    fn clear_timeout(&mut self) {}
    fn pending_timeout(&self) -> bool { false }
}

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
    performer: Processor<DummySyncHandler>,
    inner: TerminalInner<D>,
}

pub struct TerminalInner<D: DrawTarget> {
    cursor: Cursor,
    saved_cursor: Cursor,
    alt_cursor: Cursor,
    mode: TerminalMode,
    attribute_template: Cell,
    buffer: TerminalBuffer<D>,
    keyboard: KeyboardManager,
    mouse: MouseManager,
    scroll_region: Range<usize>,
}

impl<D: DrawTarget> Terminal<D> {
    pub fn new(display: D) -> Self {
        let mut graphic = Graphic::new(display);
        graphic.clear(Cell::default());

        Self {
            performer: Processor::new(),
            inner: TerminalInner {
                cursor: Cursor::default(),
                saved_cursor: Cursor::default(),
                alt_cursor: Cursor::default(),
                mode: TerminalMode::default(),
                attribute_template: Cell::default(),
                buffer: TerminalBuffer::new(graphic),
                keyboard: KeyboardManager::default(),
                mouse: MouseManager::default(),
                scroll_region: Default::default(),
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

    pub fn process(&mut self, bstr: &[u8]) {
        self.inner.cursor_handler(false);
        self.performer.advance(&mut self.inner, bstr);
        if self.inner.mode.contains(TerminalMode::SHOW_CURSOR) {
            self.inner.cursor_handler(true);
        }
        if CONFIG.auto_flush.load(Ordering::Relaxed) {
            self.flush();
        }
    }
}

impl<D: DrawTarget> Terminal<D> {
    pub fn handle_keyboard(&mut self, scancode: u8) -> Option<String> {
        let event = self.inner.keyboard.handle_keyboard(scancode);

        if let KeyboardEvent::AnsiString(s) = event {
            if !self.inner.buffer.is_latest() {
                self.inner.buffer.goto_latest();
            }
            return Some(s);
        }

        match event {
            KeyboardEvent::SetColorScheme(index) => self.set_color_scheme(index),
            KeyboardEvent::ScrollUp => self.inner.scroll_history(-1),
            KeyboardEvent::ScrollDown => self.inner.scroll_history(1),
            KeyboardEvent::ScrollPageUp => self.inner.scroll_history(-(self.rows() as isize)),
            KeyboardEvent::ScrollPageDown => self.inner.scroll_history(self.rows() as isize),
            _ => {}
        }
        None
    }

    pub fn handle_mouse(&mut self, input: MouseInput) -> Option<String> {
        if !self.inner.mode.contains(TerminalMode::ALT_SCREEN) {
            match self.inner.mouse.handle_mouse(input) {
                MouseEvent::Scroll(lines) => self.inner.scroll_history(lines),
                _ => {}
            }
            return None;
        }

        match self.inner.mouse.handle_mouse(input) {
            MouseEvent::Scroll(lines) => {
                let key = if lines > 0 {
                    KeyCode::ArrowUp
                } else {
                    KeyCode::ArrowDown
                };
                (0..lines.unsigned_abs())
                    .flat_map(|_| self.inner.keyboard.simulate_key(key))
                    .collect::<String>()
                    .into()
            }
            _ => None,
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

    pub fn set_bell_handler(&mut self, handler: Option<fn()>) {
        *CONFIG.bell_handler.lock() = handler;
    }

    pub fn set_history_size(&mut self, size: usize) {
        self.inner.buffer.resize_history(size);
    }

    pub fn set_scroll_speed(&mut self, speed: usize) {
        self.inner.mouse.set_scroll_speed(speed);
    }

    pub fn set_auto_crnl(&mut self, auto_crnl: bool) {
        CONFIG.auto_crnl.store(auto_crnl, Ordering::Relaxed);
    }

    pub fn set_font_manager(&mut self, font_manager: Box<dyn FontManager>) {
        let (font_width, font_height) = font_manager.size();
        self.inner.buffer.update_size(font_width, font_height);
        self.inner.scroll_region = 0..self.inner.buffer.height() - 1;
        self.inner.reset_state();
        *CONFIG.font_manager.lock() = Some(font_manager);
    }

    pub fn set_color_scheme(&mut self, palette_index: usize) {
        *CONFIG.color_scheme.lock() = ColorScheme::new(palette_index);
        self.inner.attribute_template = Cell::default();
        self.inner.buffer.full_flush();
    }

    pub fn set_custom_color_scheme(&mut self, palette: &Palette) {
        *CONFIG.color_scheme.lock() = ColorScheme::from(palette);
        self.inner.attribute_template = Cell::default();
        self.inner.buffer.full_flush();
    }
}

impl<D: DrawTarget> fmt::Write for Terminal<D> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.process(s.as_bytes());
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
            CursorShape::HollowBlock => Flags::CURSOR_BLOCK,
            CursorShape::Hidden => Flags::HIDDEN,
        };

        if enable {
            origin_cell.flags.insert(flag);
        } else {
            origin_cell.flags.remove(flag);
        }

        self.buffer.write(row, column, origin_cell);
    }

    fn scroll_history(&mut self, count: isize) {
        self.buffer.scroll_history(count);
        if CONFIG.auto_flush.load(Ordering::Relaxed) {
            self.buffer.flush();
        }
    }

    fn swap_alt_screen(&mut self) {
        self.mode ^= TerminalMode::ALT_SCREEN;
        swap(&mut self.cursor, &mut self.alt_cursor);
        self.buffer.swap_alt_screen(self.attribute_template);

        if !self.mode.contains(TerminalMode::ALT_SCREEN) {
            self.saved_cursor = self.cursor;
            self.attribute_template = Cell::default();
        }
    }
}

impl<D: DrawTarget> Handler for TerminalInner<D> {
    fn set_title(&mut self, title: Option<String>) {
        log!("Unhandled set_title: {:?}", title);
    }

    fn set_cursor_style(&mut self, style: Option<CursorStyle>) {
        log!("Set cursor style: {:?}", style);
        if let Some(style) = style {
            self.set_cursor_shape(style.shape);
        }
    }

    fn set_cursor_shape(&mut self, shape: CursorShape) {
        log!("Set cursor shape: {:?}", shape);
        self.cursor.shape = shape;
    }

    fn input(&mut self, content: char) {
        let template = self.attribute_template.set_content(content);
        let width = if template.wide { 2 } else { 1 };

        if self.cursor.column + width > self.buffer.width() {
            if !self.mode.contains(TerminalMode::LINE_WRAP) {
                return;
            }
            self.linefeed();
            self.carriage_return();
        }

        self.buffer
            .write(self.cursor.row, self.cursor.column, template);
        self.cursor.column += 1;

        if template.wide {
            self.buffer.write(
                self.cursor.row,
                self.cursor.column,
                template.set_placeholder(),
            );
            self.cursor.column += 1;
        }
    }

    fn goto(&mut self, row: i32, col: usize) {
        self.cursor.row = min(row as usize, self.buffer.height());
        self.cursor.column = min(col, self.buffer.width());
    }

    fn goto_line(&mut self, row: i32) {
        log!("Goto line: {}", row);
        self.goto(row, self.cursor.column);
    }

    fn goto_col(&mut self, col: usize) {
        log!("Goto column: {}", col);
        self.goto(self.cursor.row as i32, col);
    }

    fn insert_blank(&mut self, count: usize) {
        log!("Insert blank: {}", count);
        let (row, columns) = (self.cursor.row, self.buffer.width());
        let count = min(count, columns - self.cursor.column);

        let template = self.attribute_template.clear();
        for column in (self.cursor.column..columns - count).rev() {
            self.buffer
                .write(row, column + count, self.buffer.read(row, column));
            self.buffer.write(row, column, template);
        }
    }

    fn move_up(&mut self, rows: usize) {
        log!("Move up: {}", rows);
        self.goto(
            self.cursor.row.saturating_sub(rows) as i32,
            self.cursor.column,
        );
    }

    fn move_down(&mut self, rows: usize) {
        log!("Move down: {}", rows);
        let goto_line = min(self.cursor.row + rows, self.buffer.height() - 1) as i32;
        self.goto(goto_line, self.cursor.column);
    }

    fn identify_terminal(&mut self, intermediate: Option<char>) {
        log!("Unhandled identify terminal: {:?}", intermediate);
    }

    fn device_status(&mut self, status: usize) {
        log!("Unhandled device_status: {}", status);
    }

    fn move_forward(&mut self, cols: usize) {
        log!("Move forward: {}", cols);
        self.cursor.column = min(self.cursor.column + cols, self.buffer.width() - 1);
    }

    fn move_backward(&mut self, cols: usize) {
        log!("Move backward: {}", cols);
        self.cursor.column = self.cursor.column.saturating_sub(cols);
    }

    fn move_up_and_cr(&mut self, rows: usize) {
        log!("Move up and cr: {}", rows);
        self.goto(self.cursor.row.saturating_sub(rows) as i32, 0);
    }

    fn move_down_and_cr(&mut self, rows: usize) {
        log!("Move down and cr: {}", rows);
        let goto_line = min(self.cursor.row + rows, self.buffer.height() - 1);
        self.goto(goto_line as i32, 0);
    }

    fn put_tab(&mut self, count: u16) {
        log!("Put tab: {}", count);
        for _ in 0..count {
            let tab_stop = self.cursor.column.div_ceil(8) * 8;
            let end_column = tab_stop.min(self.buffer.width());
            let template = self.attribute_template.clear();

            while self.cursor.column < end_column {
                self.buffer
                    .write(self.cursor.row, self.cursor.column, template);
                self.cursor.column += 1;
            }
        }
    }

    fn backspace(&mut self) {
        self.cursor.column = self.cursor.column.saturating_sub(1);
    }

    fn carriage_return(&mut self) {
        self.cursor.column = 0;
    }

    fn linefeed(&mut self) {
        if CONFIG.auto_crnl.load(Ordering::Relaxed) {
            self.carriage_return();
        }

        if self.cursor.row == self.scroll_region.end {
            self.scroll_up(1);
        } else if self.cursor.row < self.buffer.height() - 1 {
            self.cursor.row += 1;
        }
    }

    fn bell(&mut self) {
        log!("Bell triggered!");
        CONFIG.bell_handler.lock().map(|handler| handler());
    }

    fn substitute(&mut self) {
        log!("Unhandled substitute!");
    }

    fn newline(&mut self) {
        log!("Newline!");
        self.linefeed();

        if self.mode.contains(TerminalMode::LINE_FEED_NEW_LINE) {
            self.carriage_return();
        }
    }

    fn set_horizontal_tabstop(&mut self) {
        log!("Unhandled set horizontal tabstop!");
    }

    fn scroll_up(&mut self, count: usize) {
        self.buffer.scroll_region(
            -(count as isize),
            self.attribute_template,
            self.scroll_region.clone(),
        );
    }

    fn scroll_down(&mut self, count: usize) {
        self.buffer.scroll_region(
            count as isize,
            self.attribute_template,
            self.scroll_region.clone(),
        );
    }

    fn insert_blank_lines(&mut self, count: usize) {
        log!("Insert blank lines: {}", count);
        self.scroll_down(count);
    }

    fn delete_lines(&mut self, count: usize) {
        log!("Delete lines: {}", count);
        self.scroll_up(count);
    }

    fn erase_chars(&mut self, count: usize) {
        log!("Erase chars: {}", count);
        let start = self.cursor.column;
        let end = min(start + count, self.buffer.width());

        let template = self.attribute_template.clear();
        for column in start..end {
            self.buffer.write(self.cursor.row, column, template);
        }
    }

    fn delete_chars(&mut self, count: usize) {
        log!("Delete chars: {}", count);
        let (row, columns) = (self.cursor.row, self.buffer.width());
        let count = min(count, columns - self.cursor.column - 1);

        let template = self.attribute_template.clear();
        for column in (self.cursor.column + count)..columns {
            self.buffer
                .write(row, column - count, self.buffer.read(row, column));
            self.buffer.write(row, column, template);
        }
    }

    fn move_backward_tabs(&mut self, count: u16) {
        log!("Unhandled move backward tabs: {}", count);
    }

    fn move_forward_tabs(&mut self, count: u16) {
        log!("Unhandled move forward tabs: {}", count);
    }

    fn save_cursor_position(&mut self) {
        log!("Save cursor position");
        self.saved_cursor = self.cursor;
    }

    fn restore_cursor_position(&mut self) {
        log!("Restore cursor position");
        self.cursor = self.saved_cursor;
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        log!("Clear line: {:?}", mode);
        let template = self.attribute_template.clear();
        match mode {
            LineClearMode::Right => {
                for column in self.cursor.column..self.buffer.width() {
                    self.buffer.write(self.cursor.row, column, template);
                }
            }
            LineClearMode::Left => {
                for column in 0..=self.cursor.column {
                    self.buffer.write(self.cursor.row, column, template);
                }
            }
            LineClearMode::All => {
                for column in 0..self.buffer.width() {
                    self.buffer.write(self.cursor.row, column, template);
                }
            }
        }
    }

    fn clear_screen(&mut self, mode: ClearMode) {
        log!("Clear screen: {:?}", mode);
        let template = self.attribute_template.clear();
        match mode {
            ClearMode::Above => {
                for row in 0..self.cursor.row {
                    for column in 0..self.buffer.width() {
                        self.buffer.write(row, column, template);
                    }
                }
                for column in 0..=self.cursor.column {
                    self.buffer.write(self.cursor.row, column, template);
                }
            }
            ClearMode::Below => {
                for column in self.cursor.column..self.buffer.width() {
                    self.buffer.write(self.cursor.row, column, template);
                }
                for row in self.cursor.row + 1..self.buffer.height() {
                    for column in 0..self.buffer.width() {
                        self.buffer.write(row, column, template);
                    }
                }
            }
            ClearMode::All => {
                self.buffer.clear(template);
                self.cursor = Cursor::default();
            }
            ClearMode::Saved => {
                self.buffer.clear(template);
                self.cursor = Cursor::default();
                self.buffer.clear_history();
            }
        }
    }

    fn clear_tabs(&mut self, mode: TabulationClearMode) {
        log!("Unhandled clear tabs: {:?}", mode);
    }

    fn reset_state(&mut self) {
        log!("Reset state");
        if self.mode.contains(TerminalMode::ALT_SCREEN) {
            self.swap_alt_screen();
        }
        self.buffer.clear(Cell::default());
        self.cursor = Cursor::default();
        self.saved_cursor = self.cursor;
        self.buffer.clear_history();
        self.mode = TerminalMode::default();
        self.attribute_template = Cell::default();
    }

    fn reverse_index(&mut self) {
        log!("Reverse index");
        if self.cursor.row == self.scroll_region.start {
            self.scroll_down(1);
        } else {
            self.cursor.row -= 1;
        }
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        let handle_color = |color: AnsiColor| match color {
            AnsiColor::Named(color) => Color::Indexed(color as u16),
            AnsiColor::Spec(color) => Color::Rgb((color.r, color.g, color.b)),
            AnsiColor::Indexed(index) => Color::Indexed(index as u16),
        };

        match attr {
            Attr::Foreground(color) => self.attribute_template.foreground = handle_color(color),
            Attr::Background(color) => self.attribute_template.background = handle_color(color),
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
            _ => log!("Unhandled terminal attribute: {:?}", attr),
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        let mode = match mode {
            Mode::Named(mode) => mode,
            Mode::Unknown(mode) => {
                log!("Ignoring unknown mode {} in set_mode", mode);
                return;
            }
        };

        match mode {
            NamedMode::Insert => self.mode.insert(TerminalMode::INSERT),
            NamedMode::LineFeedNewLine => self.mode.insert(TerminalMode::LINE_FEED_NEW_LINE),
        }
    }

    fn unset_mode(&mut self, mode: Mode) {
        let mode = match mode {
            Mode::Named(mode) => mode,
            Mode::Unknown(mode) => {
                log!("Ignoring unknown mode {} in unset_mode", mode);
                return;
            }
        };

        match mode {
            NamedMode::Insert => self.mode.remove(TerminalMode::INSERT),
            NamedMode::LineFeedNewLine => self.mode.remove(TerminalMode::LINE_FEED_NEW_LINE),
        }
    }

    fn report_mode(&mut self, mode: Mode) {
        log!("Unhandled report mode: {:?}", mode);
    }

    fn set_private_mode(&mut self, mode: PrivateMode) {
        let mode = match mode {
            PrivateMode::Named(mode) => mode,
            PrivateMode::Unknown(mode) => {
                log!("Ignoring unknown mode {} in set_private_mode", mode);
                return;
            }
        };

        match mode {
            NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                if !self.mode.contains(TerminalMode::ALT_SCREEN) {
                    self.swap_alt_screen();
                }
            }
            NamedPrivateMode::ShowCursor => self.mode.insert(TerminalMode::SHOW_CURSOR),
            NamedPrivateMode::CursorKeys => {
                self.mode.insert(TerminalMode::APP_CURSOR);
                self.keyboard.set_app_cursor(true);
            }
            NamedPrivateMode::LineWrap => self.mode.insert(TerminalMode::LINE_WRAP),
            _ => log!("Unhandled set mode: {:?}", mode),
        }
    }

    fn unset_private_mode(&mut self, mode: PrivateMode) {
        let mode = match mode {
            PrivateMode::Named(mode) => mode,
            PrivateMode::Unknown(mode) => {
                log!("Ignoring unknown mode {} in unset_private_mode", mode);
                return;
            }
        };

        match mode {
            NamedPrivateMode::SwapScreenAndSetRestoreCursor => {
                if self.mode.contains(TerminalMode::ALT_SCREEN) {
                    self.swap_alt_screen();
                }
            }
            NamedPrivateMode::ShowCursor => self.mode.remove(TerminalMode::SHOW_CURSOR),
            NamedPrivateMode::CursorKeys => {
                self.mode.remove(TerminalMode::APP_CURSOR);
                self.keyboard.set_app_cursor(false);
            }
            NamedPrivateMode::LineWrap => self.mode.remove(TerminalMode::LINE_WRAP),
            _ => log!("Unhandled unset mode: {:?}", mode),
        }
    }

    fn report_private_mode(&mut self, mode: PrivateMode) {
        log!("Unhandled report private mode: {:?}", mode);
    }

    fn set_scrolling_region(&mut self, top: usize, bottom: Option<usize>) {
        log!("Set scrolling region: top={}, bottom={:?}", top, bottom);
        let bottom = bottom.unwrap_or(self.buffer.height());

        if top >= bottom {
            log!("Invalid scrolling region: ({};{})", top, bottom);
            return;
        }

        self.scroll_region.start = min(top, self.buffer.height()) - 1;
        self.scroll_region.end = min(bottom, self.buffer.height()) - 1;
        self.goto(0, 0);
    }

    fn set_keypad_application_mode(&mut self) {
        log!("Set keypad application mode");
        self.mode.insert(TerminalMode::APP_KEYPAD);
    }

    fn unset_keypad_application_mode(&mut self) {
        log!("Unset keypad application mode");
        self.mode.remove(TerminalMode::APP_KEYPAD);
    }

    fn set_active_charset(&mut self, index: CharsetIndex) {
        log!("Unhandled set active charset: {:?}", index);
    }

    fn configure_charset(&mut self, index: CharsetIndex, charset: StandardCharset) {
        log!("Unhandled configure charset: {:?}, {:?}", index, charset);
    }

    fn set_color(&mut self, index: usize, color: Rgb) {
        log!("Unhandled set color: {}, {:?}", index, color);
    }

    fn dynamic_color_sequence(&mut self, prefix: String, index: usize, terminator: &str) {
        log!(
            "Unhandled dynamic color sequence: {}, {}, {}",
            prefix,
            index,
            terminator
        );
    }

    fn reset_color(&mut self, index: usize) {
        log!("Unhandled reset color: {}", index);
    }

    fn clipboard_store(&mut self, clipboard: u8, base64: &[u8]) {
        log!("Unhandled clipboard store: {}, {:?}", clipboard, base64);
    }

    fn clipboard_load(&mut self, clipboard: u8, terminator: &str) {
        log!("Unhandled clipboard load: {}, {}", clipboard, terminator);
    }

    fn decaln(&mut self) {
        log!("Unhandled decaln!");
    }

    fn push_title(&mut self) {
        log!("Unhandled push title!");
    }

    fn pop_title(&mut self) {
        log!("Unhandled pop title!");
    }

    fn text_area_size_pixels(&mut self) {
        log!("Unhandled text area size pixels!");
    }

    fn text_area_size_chars(&mut self) {
        log!("Unhandled text area size chars!");
    }

    fn set_hyperlink(&mut self, hyperlink: Option<Hyperlink>) {
        log!("Unhandled set hyperlink: {:?}", hyperlink);
    }

    fn report_keyboard_mode(&mut self) {
        log!("Unhandled report keyboard mode!");
    }

    fn push_keyboard_mode(&mut self, mode: KeyboardModes) {
        log!("Unhandled push keyboard mode: {:?}", mode);
    }

    fn pop_keyboard_modes(&mut self, to_pop: u16) {
        log!("Unhandled pop keyboard modes: {}", to_pop);
    }
}
