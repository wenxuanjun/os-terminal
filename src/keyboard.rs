use alloc::string::{String, ToString};
use pc_keyboard::layouts::Us104Key;
use pc_keyboard::KeyCode::{self, *};
use pc_keyboard::{DecodedKey, Keyboard};
use pc_keyboard::{HandleControl, ScancodeSet1};

pub enum KeyboardEvent {
    AnsiString(String),
    SetColorScheme(usize),
    ScrollUp,
    ScrollDown,
    ScrollPageUp,
    ScrollPageDown,
    None,
}

pub struct KeyboardManager {
    app_cursor_mode: bool,
    natural_scroll: bool,
    keyboard: Keyboard<Us104Key, ScancodeSet1>,
}

impl Default for KeyboardManager {
    fn default() -> Self {
        Self {
            app_cursor_mode: false,
            natural_scroll: true,
            keyboard: Keyboard::new(
                ScancodeSet1::new(),
                Us104Key,
                HandleControl::MapLettersToUnicode,
            ),
        }
    }
}

impl KeyboardManager {
    pub fn set_app_cursor(&mut self, mode: bool) {
        self.app_cursor_mode = mode;
    }

    pub fn set_natural_scroll(&mut self, mode: bool) {
        self.natural_scroll = mode;
    }

    pub fn handle_keyboard(&mut self, scancode: u8) -> KeyboardEvent {
        if let Some(key_event) = self.keyboard.add_byte(scancode).ok().flatten() {
            if let Some(decoded_key) = self.keyboard.process_keyevent(key_event) {
                return self.key_to_ansi_string(decoded_key);
            }
        }
        KeyboardEvent::None
    }
}

impl KeyboardManager {
    #[rustfmt::skip]
    fn key_to_ansi_string(&self, key: DecodedKey) -> KeyboardEvent {
        let modifiers = self.keyboard.get_modifiers();

        match key {
            DecodedKey::Unicode(c) => {
                KeyboardEvent::AnsiString(c.to_string())
            }
            DecodedKey::RawKey(key) => {
                if modifiers.is_ctrl() && modifiers.is_shifted() {
                    match (key, self.natural_scroll) {
                        (ArrowUp, true) | (ArrowDown, false) => return KeyboardEvent::ScrollUp,
                        (ArrowUp, false) | (ArrowDown, true) => return KeyboardEvent::ScrollDown,
                        (PageUp, _) => return KeyboardEvent::ScrollPageUp,
                        (PageDown, _) => return KeyboardEvent::ScrollPageDown,
                        _ => {},
                    };

                    let palette_index = match key {
                        KeyCode::F1 => Some(0),
                        KeyCode::F2 => Some(1),
                        KeyCode::F3 => Some(2),
                        KeyCode::F4 => Some(3),
                        KeyCode::F5 => Some(4),
                        KeyCode::F6 => Some(5),
                        KeyCode::F7 => Some(6),
                        KeyCode::F8 => Some(7),
                        _ => None,
                    };
                    if let Some(palette_index) = palette_index {
                        return KeyboardEvent::SetColorScheme(palette_index as usize);
                    }
                }

                let sequence = match key {
                    F1 => "\x1bOP",
                    F2 => "\x1bOQ",
                    F3 => "\x1bOR",
                    F4 => "\x1bOS",
                    F5 => "\x1b[15~",
                    F6 => "\x1b[17~",
                    F7 => "\x1b[18~",
                    F8 => "\x1b[19~",
                    F9 => "\x1b[20~",
                    F10 => "\x1b[21~",
                    F11 => "\x1b[23~",
                    F12 => "\x1b[24~",
                    ArrowUp => if self.app_cursor_mode { "\x1bOA" } else { "\x1b[A" },
                    ArrowDown => if self.app_cursor_mode { "\x1bOB" } else { "\x1b[B" },
                    ArrowRight => if self.app_cursor_mode { "\x1bOC" } else { "\x1b[C" },
                    ArrowLeft => if self.app_cursor_mode { "\x1bOD" } else { "\x1b[D" },
                    Home => "\x1b[H",
                    End => "\x1b[F",
                    PageUp => "\x1b[5~",
                    PageDown => "\x1b[6~",
                    _ => "",
                };

                match sequence {
                    "" => KeyboardEvent::None,
                    _ => KeyboardEvent::AnsiString(sequence.to_string()),
                }
            }
        }
    }
}
