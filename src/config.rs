use alloc::boxed::Box;
use core::fmt;
use spin::{Lazy, Mutex};

use crate::font::FontManager;

pub static CONFIG: Lazy<Mutex<TerminalConfig>> =
    Lazy::new(|| Mutex::new(TerminalConfig::default()));

pub struct TerminalConfig {
    pub auto_flush: bool,
    pub logger: Option<fn(fmt::Arguments)>,
    pub font_manager: Option<Box<dyn FontManager>>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            auto_flush: true,
            logger: None,
            font_manager: None,
        }
    }
}
