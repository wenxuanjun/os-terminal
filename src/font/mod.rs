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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontWeight {
    Regular,
    Bold,
}

pub trait FontManager {
    fn size(&self) -> (usize, usize);
    fn rasterize(&mut self, content: char, weight: FontWeight) -> Rasterized;
}
