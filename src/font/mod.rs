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
    Owned(Vec<Vec<u8>>)
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentInfo {
    pub content: char,
    pub bold: bool,
    pub italic: bool,
    pub wide: bool,
}

impl ContentInfo {
    pub fn new(content: char, bold: bool, italic: bool, wide: bool) -> Self {
        Self {
            content,
            bold,
            italic,
            wide,
        }
    }
}

pub trait FontManager: Send {
    fn size(&self) -> (usize, usize);
    fn rasterize(&mut self, info: ContentInfo) -> Rasterized;
}
