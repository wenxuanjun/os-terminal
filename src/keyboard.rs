use alloc::string::{String, ToString};
use pc_keyboard::layouts::Us104Key;
use pc_keyboard::KeyCode::{self, *};
use pc_keyboard::{DecodedKey, Keyboard};
use pc_keyboard::{HandleControl, ScancodeSet1};

#[derive(Debug)]
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
    keyboard: Keyboard<Us104Key, ScancodeSet1>,
}

impl Default for KeyboardManager {
    fn default() -> Self {
        Self {
            app_cursor_mode: false,
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

    pub fn simulate_key(&mut self, key: KeyCode) -> Option<String> {
        match self.key_to_event(DecodedKey::RawKey(key)) {
            KeyboardEvent::AnsiString(s) => Some(s),
            _ => None,
        }
    }

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
    fn key_to_event(&self, key: DecodedKey) -> KeyboardEvent {
        let modifiers = self.keyboard.get_modifiers();

        match key {
            DecodedKey::RawKey(key) => {
                if modifiers.is_ctrl() && modifiers.is_shifted() {
                    if let Some(event) = self
                        .handle_scroll(key)
                        .or_else(|| self.handle_color_scheme(key))
                    {
                        return event;
                    }
                }

                self.generate_ansi_sequence(key)
                    .map(|s| KeyboardEvent::AnsiString(s.to_string()))
                    .unwrap_or(KeyboardEvent::None)
            }
            DecodedKey::Unicode(c) => KeyboardEvent::AnsiString(c.to_string()),
        }
    }

    fn handle_color_scheme(&self, key: KeyCode) -> Option<KeyboardEvent> {
        let index = match key {
            F1 => 0,
            F2 => 1,
            F3 => 2,
            F4 => 3,
            F5 => 4,
            F6 => 5,
            F7 => 6,
            F8 => 7,
            _ => return None,
        };
        Some(KeyboardEvent::SetColorScheme(index))
    }

    fn handle_scroll(&self, key: KeyCode) -> Option<KeyboardEvent> {
        match key {
            ArrowUp => Some(KeyboardEvent::ScrollUp),
            ArrowDown => Some(KeyboardEvent::ScrollDown),
            PageUp => Some(KeyboardEvent::ScrollPageUp),
            PageDown => Some(KeyboardEvent::ScrollPageDown),
            _ => None,
        }
    }

    #[rustfmt::skip]
    fn generate_ansi_sequence(&self, key: KeyCode) -> Option<&'static str>{
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
