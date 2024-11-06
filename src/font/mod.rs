use alloc::vec::Vec;

#[cfg(feature = "bitmap")]
mod bitmap;
#[cfg(feature = "truetype")]
mod truetype;

#[cfg(feature = "bitmap")]
pub use bitmap::BitmapFont;
#[cfg(feature = "truetype")]
pub use truetype::TrueTypeFont;

pub enum Rasterized<'a> {
    Slice(&'a [&'a [u8]]),
    Vec(&'a Vec<Vec<u8>>),
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentInfo {
    content: char,
    bold: bool,
    italic: bool,
    width_ratio: usize,
}

impl ContentInfo {
    pub fn new(content: char, bold: bool, italic: bool, width_ratio: usize) -> Self {
        Self {
            content,
            bold,
            italic,
            width_ratio,
        }
    }
}

pub trait FontManager: Send {
    fn size(&self) -> (usize, usize);
    fn rasterize(&mut self, info: ContentInfo) -> Rasterized;
}
