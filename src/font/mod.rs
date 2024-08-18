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
    Owned(Vec<Vec<u8>>),
    Borrowed(&'a [&'a [u8]]),
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentInfo {
    content: char,
    bold: bool,
    italic: bool,
}

impl ContentInfo {
    pub fn new(content: char, bold: bool, italic: bool) -> Self {
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
