use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "bitmap")]
mod bitmap;
#[cfg(feature = "truetype")]
mod truetype;

#[cfg(feature = "bitmap")]
pub use bitmap::BitmapFont;
#[cfg(feature = "truetype")]
pub use truetype::TrueTypeFont;

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentInfo {
    content: char,
    bold: bool,
    italic: bool,
}

impl ContentInfo {
    pub const fn new(content: char, bold: bool, italic: bool) -> Self {
        Self {
            content,
            bold,
            italic,
        }
    }
}

pub trait FontManager: Send {
    fn size(&self) -> (usize, usize);
    fn rasterize(&mut self, info: ContentInfo) -> Rasterized;
}

pub enum Rasterized<'a> {
    Slice(&'a [&'a [u8]]),
    Vec(Vec<Vec<u8>>),
}

impl<'a> Rasterized<'a> {
    pub fn as_2d_array(&'a self) -> Box<dyn Iterator<Item = &'a [u8]> + 'a> {
        match self {
            Self::Slice(slice) => Box::new(slice.iter().copied()),
            Self::Vec(vec) => Box::new(vec.iter().map(|row| row.as_slice())),
        }
    }
}
