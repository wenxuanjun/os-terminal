use crate::config::CONFIG;
use core::fmt;

macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::log_message(format_args!($($arg)*))
    };
}

pub fn log_message(args: fmt::Arguments) {
    if let Some(logger) = CONFIG.logger.lock().as_ref() {
        logger(args);
    }
}
