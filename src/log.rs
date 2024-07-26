use core::fmt;
use spin::Mutex;

static LOGGER: Mutex<Option<fn(fmt::Arguments)>> = Mutex::new(None);

pub fn set_logger(logger: fn(fmt::Arguments)) {
    let mut log = LOGGER.lock();
    *log = Some(logger);
}

macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::log_message(format_args!($($arg)*))
    };
}

pub(crate) use log;

pub(crate) fn log_message(args: fmt::Arguments) {
    if let Some(logger) = *LOGGER.lock() {
        logger(args);
    }
}
