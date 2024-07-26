#![no_std]

mod ansi;
mod buffer;
mod cell;
mod color;
mod console;
mod graphic;
mod log;

extern crate alloc;

pub use console::Console;
pub use graphic::DrawTarget;
pub use log::set_logger;
