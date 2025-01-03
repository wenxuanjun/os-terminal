use alloc::boxed::Box;
use core::{fmt, sync::atomic::AtomicBool};
use spin::{Lazy, Mutex};

use crate::color::ColorScheme;
use crate::font::FontManager;

pub static CONFIG: Lazy<TerminalConfig> = Lazy::new(TerminalConfig::default);

pub struct TerminalConfig {
    pub auto_flush: AtomicBool,
    pub logger: Mutex<Option<fn(fmt::Arguments)>>,
    pub font_manager: Mutex<Option<Box<dyn FontManager>>>,
    pub color_scheme: Mutex<ColorScheme>,
    pub bell_handler: Mutex<Option<fn()>>,
    pub auto_crnl: AtomicBool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            auto_flush: AtomicBool::new(true),
            logger: Mutex::new(None),
            font_manager: Mutex::new(None),
            color_scheme: Mutex::new(ColorScheme::default()),
            bell_handler: Mutex::new(None),
            auto_crnl: AtomicBool::new(true),
        }
    }
}
