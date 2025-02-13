#![no_std]
#![forbid(unsafe_code)]

#[macro_use]
extern crate alloc;

#[macro_use]
mod log;

mod buffer;
mod cell;
mod color;
mod config;
mod graphic;
mod keyboard;
mod mouse;
mod palette;
mod terminal;

pub mod font;

pub use color::Rgb;
pub use graphic::DrawTarget;
pub use keyboard::KeyboardManager;
pub use mouse::{MouseButton, MouseInput};
pub use palette::Palette;
pub use terminal::Terminal;
