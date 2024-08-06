use alloc::boxed::Box;
use core::fmt;
use spin::{Lazy, Mutex};

use crate::font::FontManager;

pub static CONFIG: Lazy<Mutex<TerminalConfig>> =
    Lazy::new(|| Mutex::new(TerminalConfig::default()));

pub type FontManagerRef = Box<dyn FontManager + Send>;

pub struct TerminalConfig {
    pub auto_flush: bool,
    pub logger: Option<fn(fmt::Arguments)>,
    pub font_manager: Option<FontManagerRef>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        TerminalConfig {
            auto_flush: true,
            logger: None,
            font_manager: None,
        }
    }
}
