#![no_std]
#![forbid(unsafe_code)]
#![allow(clippy::option_map_unit_fn)]

#[macro_use]
extern crate alloc;

mod buffer;
mod cell;
mod color;
mod graphic;
mod keyboard;
mod mouse;
mod palette;
mod terminal;
#[cfg(feature = "wallpaper")]
mod wallpaper;

pub mod font;

pub use color::Rgb;
pub use graphic::DrawTarget;
pub use keyboard::KeyboardEvent;
pub use mouse::{MouseButton, MouseInput};
pub use palette::Palette;
pub use terminal::{ClipboardHandler, Terminal};
#[cfg(feature = "wallpaper")]
pub use wallpaper::WallpaperError;
