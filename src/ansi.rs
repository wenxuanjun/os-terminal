use vte::{Params, ParamsIter, Perform};

use super::cell::Cell;
use super::color::{Color, NamedColor};

#[derive(Debug)]
pub enum LineClearMode {
    Right,
    Left,
    All,
}

#[derive(Debug)]
pub enum ScreenClearMode {
    Below,
    Above,
    All,
    Saved,
}

#[derive(Debug)]
pub enum Attr {
    Reset,
    Bold,
    Underline,
    Reverse,
    Hidden,
    CancelBold,
    CancelBoldDim,
    CancelUnderline,
    CancelReverse,
    CancelHidden,
    Foreground(Color),
    Background(Color),
}

pub trait Handler {
    fn input(&mut self, _content: char) {}
    fn goto(&mut self, _row: usize, _col: usize) {}
    fn goto_line(&mut self, _row: usize) {}
    fn goto_column(&mut self, _col: usize) {}
    fn move_up(&mut self, _rows: usize) {}
    fn move_down(&mut self, _rows: usize) {}
    fn move_forward(&mut self, _cols: usize) {}
    fn move_backward(&mut self, _cols: usize) {}
    fn move_down_and_cr(&mut self, _rows: usize) {}
    fn move_up_and_cr(&mut self, _rows: usize) {}
    fn put_tab(&mut self) {}
    fn backspace(&mut self) {}
    fn carriage_return(&mut self) {}
    fn linefeed(&mut self) {}
    fn erase_chars(&mut self, _count: usize) {}
    fn delete_chars(&mut self, _count: usize) {}
    fn save_cursor_position(&mut self) {}
    fn restore_cursor_position(&mut self) {}
    fn set_cursor_shape(&mut self, _shape: CursorShape) {}
    fn clear_line(&mut self, _mode: LineClearMode) {}
    fn clear_screen(&mut self, _mode: ScreenClearMode) {}
    fn terminal_attribute(&mut self, _attr: Attr) {}
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CursorShape {
    Block,
    Underline,
    Beam,
}

impl Default for CursorShape {
    fn default() -> CursorShape {
        CursorShape::Beam
    }
}

pub struct Performer<'a, H: Handler> {
    handler: &'a mut H,
}

impl<'a, H: Handler> Performer<'a, H> {
    pub fn new(handler: &'a mut H) -> Self {
        Self { handler }
    }
}

impl<'a, H: Handler> Perform for Performer<'a, H> {
    fn print(&mut self, content: char) {
        self.handler.input(content);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\x08' => self.handler.backspace(),
            b'\x09' => self.handler.put_tab(),
            b'\x0A' => self.handler.linefeed(),
            b'\x0D' => self.handler.carriage_return(),
            _ => crate::log!("Unhandled execute byte={:02x}", byte),
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() || params[0].is_empty() {
            return;
        }

        match params[0] {
            b"0" | b"2" => crate::log!("Set window title"),
            b"4" => crate::log!("Set color index"),
            b"10" | b"11" | b"12" => crate::log!("Get/set Foreground, Background, Cursor colors"),
            b"50" => {
                if params.len() >= 2
                    && params[1].len() >= 13
                    && params[1][0..12] == *b"CursorShape="
                {
                    let shape = match params[1][12] as char {
                        '0' => CursorShape::Block,
                        '1' => CursorShape::Beam,
                        '2' => CursorShape::Underline,
                        _ => {
                            crate::log!("Invalid cursor shape: {:?}", params[1]);
                            return
                        }
                    };
                    self.handler.set_cursor_shape(shape);
                }
            },
            b"52" => crate::log!("Set clipboard"),
            b"104" => crate::log!("Reset color index"),
            b"110" => crate::log!("Reset foreground color"),
            b"111" => crate::log!("Reset background color"),
            b"112" => crate::log!("Reset text cursor color"),
            _ => crate::log!("Unhandled osc_dispatch: [{:?}]", params)
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore || intermediates.len() > 1 {
            return;
        }

        let extract_one_param = |params: &Params, default: u16| {
            let mut iter = params.iter().map(|param| param[0]);
            iter.next().filter(|&param| param != 0).unwrap_or(default) as usize
        };

        let extract_two_params = |params: &Params, default: (u16, u16)| {
            let mut iter = params.iter().map(|param| param[0]);
            let first = iter.next().filter(|&param| param != 0).unwrap_or(default.0);
            let second = iter.next().filter(|&param| param != 0).unwrap_or(default.1);
            (first as usize, second as usize)
        };

        match (action, intermediates) {
            ('A', []) => self.handler.move_up(extract_one_param(params, 1)),
            ('B', []) | ('e', []) => self.handler.move_down(extract_one_param(params, 1)),
            ('C', []) | ('a', []) => self.handler.move_forward(extract_one_param(params, 1)),
            ('D', []) => self.handler.move_backward(extract_one_param(params, 1)),
            ('E', []) => self.handler.move_down_and_cr(extract_one_param(params, 1)),
            ('F', []) => self.handler.move_up_and_cr(extract_one_param(params, 1)),
            ('G', []) | ('`', []) => self.handler.goto_column(extract_one_param(params, 1) - 1),
            ('H', []) | ('f', []) => {
                let (y, x) = extract_two_params(params, (1, 1));
                self.handler.goto(y - 1, x - 1);
            }
            ('J', []) => {
                let mode = match extract_one_param(params, 0) {
                    0 => ScreenClearMode::Below,
                    1 => ScreenClearMode::Above,
                    2 => ScreenClearMode::All,
                    3 => ScreenClearMode::Saved,
                    _ => {
                        crate::log!("Invalid clear screen mode: {:?}", params);
                        return;
                    }
                };
                self.handler.clear_screen(mode);
            }
            ('K', []) => {
                let mode = match extract_one_param(params, 0) {
                    0 => LineClearMode::Right,
                    1 => LineClearMode::Left,
                    2 => LineClearMode::All,
                    _ => {
                        crate::log!("Invalid clear line mode: {:?}", params);
                        return;
                    }
                };
                self.handler.clear_line(mode);
            }
            ('P', []) => self.handler.delete_chars(extract_one_param(params, 1)),
            ('q', [b' ']) => {
                let cursor_style_id = extract_one_param(params, 0);
                let shape = match cursor_style_id {
                    0 => None,
                    1 | 2 => Some(CursorShape::Block),
                    3 | 4 => Some(CursorShape::Underline),
                    5 | 6 => Some(CursorShape::Beam),
                    _ => {
                        crate::log!("Invalid cursor style: {:?}", cursor_style_id);
                        return;
                    }
                };
                self.handler.set_cursor_shape(shape.unwrap_or(CursorShape::default()));
            },
            ('X', []) => self.handler.erase_chars(extract_one_param(params, 1)),
            ('d', []) => self.handler.goto_line(extract_one_param(params, 1) - 1),
            ('m', _) => {
                if params.is_empty() {
                    self.handler.terminal_attribute(Attr::Reset);
                } else {
                    attrs_from_sgr_parameters(&mut params.iter(), |attr| {
                        self.handler.terminal_attribute(attr)
                    });
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        macro_rules! configure_charset {
            ($intermediates:expr) => {
                crate::log!("Unhandled charset: {:?}", $intermediates)
            };
        }

        match (byte, intermediates) {
            (b'7', []) => self.handler.save_cursor_position(),
            (b'8', []) => self.handler.restore_cursor_position(),
            (b'B', intermediates) => configure_charset!(intermediates),
            _ => crate::log!("Unhandled escape code: ESC {:?} {byte}", intermediates),
        }
    }
}

fn attrs_from_sgr_parameters<F: FnMut(Attr)>(
    params: &mut ParamsIter,
    mut terminal_attribute_handler: F,
) {
    fn parse_sgr_color(params: &mut dyn Iterator<Item = u16>) -> Option<Color> {
        match params.next() {
            Some(2) => Some(Color::Rgb((
                u8::try_from(params.next()?).ok()?,
                u8::try_from(params.next()?).ok()?,
                u8::try_from(params.next()?).ok()?,
            ))),
            Some(5) => Some(Color::Indexed(u8::try_from(params.next()?).ok()?)),
            _ => None,
        }
    }

    while let Some(param) = params.next() {
        match param {
            [0] => terminal_attribute_handler(Attr::Reset),
            [1] => terminal_attribute_handler(Attr::Bold),
            [4, 0] => terminal_attribute_handler(Attr::CancelUnderline),
            [4, ..] => terminal_attribute_handler(Attr::Underline),
            [7] => terminal_attribute_handler(Attr::Reverse),
            [8] => terminal_attribute_handler(Attr::Hidden),
            [21] => terminal_attribute_handler(Attr::CancelBold),
            [22] => terminal_attribute_handler(Attr::CancelBoldDim),
            [24] => terminal_attribute_handler(Attr::CancelUnderline),
            [27] => terminal_attribute_handler(Attr::CancelReverse),
            [28] => terminal_attribute_handler(Attr::CancelHidden),
            [30..=37] => terminal_attribute_handler(Attr::Foreground(Color::Indexed(
                param[0] as u8 - 30 + NamedColor::Black as u8,
            ))),
            [38] => {
                let mut iter = params.map(|param| param[0]);
                if let Some(attr) = parse_sgr_color(&mut iter).map(Attr::Foreground) {
                    terminal_attribute_handler(attr);
                }
            }
            [38, params @ ..] => {
                let rgb_start = if params.len() > 4 { 2 } else { 1 };
                let rgb_iter = params[rgb_start..].iter().copied();
                let mut iter = core::iter::once(params[0]).chain(rgb_iter);
                if let Some(attr) = parse_sgr_color(&mut iter).map(Attr::Foreground) {
                    terminal_attribute_handler(attr);
                }
            }
            [39] => terminal_attribute_handler(Attr::Foreground(Cell::default().foreground)),
            [40..=47] => terminal_attribute_handler(Attr::Background(Color::Indexed(
                param[0] as u8 - 40 + NamedColor::Black as u8,
            ))),
            [48] => {
                let mut iter = params.map(|param| param[0]);
                if let Some(attr) = parse_sgr_color(&mut iter).map(Attr::Background) {
                    terminal_attribute_handler(attr);
                }
            }
            [48, params @ ..] => {
                let rgb_start = if params.len() > 4 { 2 } else { 1 };
                let rgb_iter = params[rgb_start..].iter().copied();
                let mut iter = core::iter::once(params[0]).chain(rgb_iter);
                if let Some(attr) = parse_sgr_color(&mut iter).map(Attr::Background) {
                    terminal_attribute_handler(attr);
                }
            }
            [49] => terminal_attribute_handler(Attr::Background(Cell::default().background)),
            [90..=97] => terminal_attribute_handler(Attr::Foreground(Color::Indexed(
                param[0] as u8 - 90 + NamedColor::BrightBlack as u8,
            ))),
            [100..=107] => terminal_attribute_handler(Attr::Background(Color::Indexed(
                param[0] as u8 - 100 + NamedColor::BrightBlack as u8,
            ))),
            _ => crate::log!("Unhandled sgr parameter: {:?}", param),
        };
    }
}
