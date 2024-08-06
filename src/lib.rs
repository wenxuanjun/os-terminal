#![no_std]
#![deny(unsafe_code)]

#[macro_use]
extern crate alloc;

#[macro_use]
mod log;

pub mod font;

mod ansi;
mod buffer;
mod cell;
mod color;
mod config;
mod graphic;
mod terminal;

pub use color::Rgb888;
pub use graphic::DrawTarget;
pub use terminal::Terminal;
