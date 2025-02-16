use alloc::boxed::Box;
use core::fmt;
use core::sync::atomic::AtomicBool;
use spin::{Lazy, Mutex};

use crate::color::ColorScheme;
use crate::font::FontManager;

pub static CONFIG: Lazy<TerminalConfig> = Lazy::new(TerminalConfig::default);

pub struct TerminalConfig {
    pub auto_flush: AtomicBool,
    pub auto_crnl: AtomicBool,
    pub logger: Mutex<Option<fn(fmt::Arguments)>>,
    pub font_manager: Mutex<Option<Box<dyn FontManager>>>,
    pub color_scheme: Mutex<ColorScheme>,
    pub bell_handler: Mutex<Option<fn()>>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            auto_flush: AtomicBool::new(true),
            auto_crnl: AtomicBool::new(true),
            logger: Mutex::default(),
            font_manager: Mutex::default(),
            color_scheme: Mutex::default(),
            bell_handler: Mutex::default(),
        }
    }
}
