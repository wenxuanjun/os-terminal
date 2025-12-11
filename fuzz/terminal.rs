#![no_main]

use os_terminal::font::{ContentInfo, FontManager, Rasterized};
use os_terminal::{DrawTarget, Rgb, Terminal};
use std::boxed::Box;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let display = DummyDisplay {
        width: 405,
        height: 300,
    };
    let mut term = Terminal::new(display);
    term.set_font_manager(Box::new(DummyFont::default()));
    term.set_auto_flush(false);
    term.process(data);
    term.flush();
});

struct DummyDisplay {
    width: usize,
    height: usize,
}

impl DrawTarget for DummyDisplay {
    fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn draw_pixel(&mut self, x: usize, y: usize, _color: Rgb) {
        if x >= self.width || y >= self.height {
            panic!(
                "Draw out of bounds: ({}, {}) limit ({}, {})",
                x, y, self.width, self.height
            );
        }
    }
}

#[derive(Default)]
struct DummyFont {
    raster: Vec<Vec<u8>>
}

impl FontManager for DummyFont {
    fn size(&self) -> (usize, usize) {
        (8, 16)
    }

    fn rasterize(&mut self, _info: ContentInfo) -> Rasterized<'_> {
        Rasterized::Vec(&self.raster)
    }
}
