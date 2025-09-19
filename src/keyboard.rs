use alloc::string::{String, ToString};
use pc_keyboard::layouts::Us104Key;
use pc_keyboard::KeyCode::{self, *};
use pc_keyboard::{DecodedKey, Keyboard};
use pc_keyboard::{HandleControl, ScancodeSet1};

#[derive(Debug)]
pub enum KeyboardEvent {
    AnsiString(String),
    Copy,
    Paste,
    SetColorScheme(usize),
    Scroll { up: bool, page: bool },
    None,
}

pub struct KeyboardManager {
    pub(crate) app_cursor_mode: bool,
    pub(crate) crnl_mapping: bool,
    keyboard: Keyboard<Us104Key, ScancodeSet1>,
}

impl Default for KeyboardManager {
    fn default() -> Self {
        Self {
            app_cursor_mode: false,
            crnl_mapping: false,
            keyboard: Keyboard::new(
                ScancodeSet1::new(),
                Us104Key,
                HandleControl::MapLettersToUnicode,
            ),
        }
    }
}

impl KeyboardManager {
    pub fn handle_keyboard(&mut self, scancode: u8) -> KeyboardEvent {
        self.keyboard
            .add_byte(scancode)
            .ok()
            .flatten()
            .and_then(|event| self.keyboard.process_keyevent(event))
            .map_or(KeyboardEvent::None, |key| self.key_to_event(key))
    }
}

impl KeyboardManager {
    pub fn key_to_event(&self, key: DecodedKey) -> KeyboardEvent {
        let modifiers = self.keyboard.get_modifiers();

        if modifiers.is_ctrl() && modifiers.is_shifted() {
            let raw_key = match key {
                DecodedKey::RawKey(k) => Some(k),
                DecodedKey::Unicode('\x03') => Some(C),
                DecodedKey::Unicode('\x16') => Some(V),
                _ => None,
            };

            if let Some(k) = raw_key {
                if let Some(event) = self.handle_function(k) {
                    return event;
                }
            }
        }

        match key {
            DecodedKey::RawKey(k) => self
                .generate_ansi_sequence(k)
                .map(|s| KeyboardEvent::AnsiString(s.to_string()))
                .unwrap_or(KeyboardEvent::None),
            DecodedKey::Unicode(c) => match c {
                '\x08' => KeyboardEvent::AnsiString("\x7f".to_string()),
                '\x7f' => KeyboardEvent::AnsiString("\x1b[3~".to_string()),
                '\n' if !self.crnl_mapping => KeyboardEvent::AnsiString("\r".to_string()),
                _ => KeyboardEvent::AnsiString(c.to_string()),
            },
        }
    }

    fn handle_function(&self, key: KeyCode) -> Option<KeyboardEvent> {
        if let Some(index) = match key {
            F1 => Some(0),
            F2 => Some(1),
            F3 => Some(2),
            F4 => Some(3),
            F5 => Some(4),
            F6 => Some(5),
            F7 => Some(6),
            F8 => Some(7),
            _ => None,
        } {
            return Some(KeyboardEvent::SetColorScheme(index));
        }

        match key {
            C => Some(KeyboardEvent::Copy),
            V => Some(KeyboardEvent::Paste),
            ArrowUp | PageUp => Some(KeyboardEvent::Scroll {
                up: true,
                page: matches!(key, PageUp),
            }),
            ArrowDown | PageDown => Some(KeyboardEvent::Scroll {
                up: false,
                page: matches!(key, PageDown),
            }),
            _ => None,
        }
    }

    #[rustfmt::skip]
    fn generate_ansi_sequence(&self, key: KeyCode) -> Option<&'static str> {
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
            _ => return None,
        };
        Some(sequence)
    }
}
