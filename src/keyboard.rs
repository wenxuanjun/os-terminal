use alloc::string::{String, ToString};
use pc_keyboard::layouts::Us104Key;
use pc_keyboard::{DecodedKey, KeyCode};
use pc_keyboard::{HandleControl, Keyboard, ScancodeSet1};

pub enum KeyboardEvent {
    AnsiString(String),
    SetColorScheme(usize),
    None,
}

pub struct KeyboardManager {
    keyboard: Keyboard<Us104Key, ScancodeSet1>,
    app_cursor_mode: bool,
}

impl Default for KeyboardManager {
    fn default() -> Self {
        Self {
            keyboard: Keyboard::new(
                ScancodeSet1::new(),
                Us104Key,
                HandleControl::MapLettersToUnicode,
            ),
            app_cursor_mode: false,
        }
    }
}

impl KeyboardManager {
    pub fn set_app_cursor(&mut self, mode: bool) {
        self.app_cursor_mode = mode;
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
                    let palette_index = match key {
                        KeyCode::F1 => Some(0),
                        KeyCode::F2 => Some(1),
                        KeyCode::F3 => Some(2),
                        KeyCode::F4 => Some(3),
                        KeyCode::F5 => Some(4),
                        KeyCode::F6 => Some(5),
                        KeyCode::F7 => Some(6),
                        KeyCode::F8 => Some(7),
                        KeyCode::F9 => Some(8),
                        KeyCode::F10 => Some(9),
                        _ => None,
                    };
                    if let Some(palette_index) = palette_index {
                        return KeyboardEvent::SetColorScheme(palette_index as usize);
                    }
                }

                let sequence = match key {
                    KeyCode::F1 => "\x1bOP",
                    KeyCode::F2 => "\x1bOQ",
                    KeyCode::F3 => "\x1bOR",
                    KeyCode::F4 => "\x1bOS",
                    KeyCode::F5 => "\x1b[15~",
                    KeyCode::F6 => "\x1b[17~",
                    KeyCode::F7 => "\x1b[18~",
                    KeyCode::F8 => "\x1b[19~",
                    KeyCode::F9 => "\x1b[20~",
                    KeyCode::F10 => "\x1b[21~",
                    KeyCode::F11 => "\x1b[23~",
                    KeyCode::F12 => "\x1b[24~",
                    KeyCode::ArrowUp => if self.app_cursor_mode { "\x1bOA" } else { "\x1b[A" },
                    KeyCode::ArrowDown => if self.app_cursor_mode { "\x1bOB" } else { "\x1b[B" },
                    KeyCode::ArrowRight => if self.app_cursor_mode { "\x1bOC" } else { "\x1b[C" },
                    KeyCode::ArrowLeft => if self.app_cursor_mode { "\x1bOD" } else { "\x1b[D" },
                    KeyCode::Home => "\x1b[H",
                    KeyCode::End => "\x1b[F",
                    KeyCode::PageUp => "\x1b[5~",
                    KeyCode::PageDown => "\x1b[6~",
                    _ => "",
                };

                KeyboardEvent::AnsiString(sequence.to_string())
            }
        }
    }
}
