use alloc::vec::Vec;
use core::{iter::Map, slice::Iter};

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
    width_ratio: usize,
}

impl ContentInfo {
    pub const fn new(content: char, bold: bool, italic: bool, width_ratio: usize) -> Self {
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

pub enum Rasterized<'a> {
    Slice(&'a [&'a [u8]]),
    Vec(Vec<Vec<u8>>),
}

impl<'a> Rasterized<'a> {
    pub fn as_2d_array(&'a self) -> RasterizedIter<'a> {
        match self {
            Self::Slice(slice) => RasterizedIter::Slice(slice.iter()),
            Self::Vec(vec) => RasterizedIter::Vec(vec.iter().map(|row| row.as_slice())),
        }
    }
}

pub enum RasterizedIter<'a> {
    Slice(Iter<'a, &'a [u8]>),
    Vec(Map<Iter<'a, Vec<u8>>, fn(&'a Vec<u8>) -> &'a [u8]>),
}

impl<'a> Iterator for RasterizedIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RasterizedIter::Slice(iter) => iter.next().copied(),
            RasterizedIter::Vec(iter) => iter.next(),
        }
    }
}
