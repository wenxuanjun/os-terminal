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
    GraySlice(&'a [&'a [u8]]),
    GrayVec(&'a Vec<Vec<u8>>),
    SubpixelVec(&'a Vec<Vec<[u8; 3]>>),
}

#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct ContentInfo {
    pub content: char,
    pub bold: bool,
    pub italic: bool,
    pub wide: bool,
}

pub trait FontManager: Send {
    fn size(&self) -> (usize, usize);
    fn rasterize(&mut self, info: ContentInfo) -> Rasterized<'_>;
}
