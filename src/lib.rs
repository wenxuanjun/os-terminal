#![no_std]
#![deny(unsafe_code)]

#[macro_use]
extern crate alloc;

#[macro_use]
mod log;

mod ansi;
mod buffer;
mod cell;
mod color;
mod config;
mod graphic;
mod keyboard;
mod terminal;

pub mod font;

pub use color::Rgb888;
pub use graphic::DrawTarget;
pub use keyboard::KeyboardManager;
pub use terminal::Terminal;
