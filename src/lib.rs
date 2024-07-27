#![no_std]

mod ansi;
mod buffer;
mod cell;
mod color;
mod graphic;
mod log;
mod terminal;

extern crate alloc;

pub use color::Rgb888;
pub use graphic::DrawTarget;
pub use log::set_logger;
pub use terminal::Terminal;
