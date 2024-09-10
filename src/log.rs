use core::fmt;

use crate::config::CONFIG;

macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::log_message(format_args!($($arg)*))
    };
}

pub fn log_message(args: fmt::Arguments) {
    if let Some(logger) = CONFIG.lock().logger {
        logger(args);
    }
}
