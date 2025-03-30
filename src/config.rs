use alloc::boxed::Box;
use alloc::string::String;
use core::fmt;
use core::sync::atomic::AtomicBool;
use spin::{Lazy, Mutex};

use crate::color::ColorScheme;
use crate::font::FontManager;

pub static CONFIG: Lazy<TerminalConfig> = Lazy::new(TerminalConfig::default);

pub trait ClipboardHandler {
    fn get_text(&mut self) -> Option<String>;
    fn set_text(&mut self, text: String);
}

pub type PtyWriter = Box<dyn Fn(String) + Send>;
pub type Clipboard = Box<dyn ClipboardHandler + Send>;

pub struct TerminalConfig {
    pub auto_flush: AtomicBool,
    pub crnl_mapping: AtomicBool,
    pub logger: Mutex<Option<fn(fmt::Arguments)>>,
    pub clipboard: Mutex<Option<Clipboard>>,
    pub pty_writer: Mutex<Option<PtyWriter>>,
    pub font_manager: Mutex<Option<Box<dyn FontManager>>>,
    pub color_scheme: Mutex<ColorScheme>,
    pub bell_handler: Mutex<Option<fn()>>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            auto_flush: AtomicBool::new(true),
            crnl_mapping: AtomicBool::new(false),
            logger: Default::default(),
            clipboard: Default::default(),
            pty_writer: Default::default(),
            font_manager: Default::default(),
            color_scheme: Default::default(),
            bell_handler: Default::default(),
        }
    }
}

impl TerminalConfig {
    pub fn pty_write(&self, data: String) {
        if let Some(writer) = self.pty_writer.lock().as_ref() {
            writer(data);
        }
    }
}
